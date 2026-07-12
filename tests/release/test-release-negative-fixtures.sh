#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
manifest=${1:-"$root/dist/release-manifest.json"}
temp=$(mktemp -d); dirty="$root/crates/harness-symphony/.us096-dirty-test-$$"; trap 'rm -rf "$temp"; rm -f "$dirty"' EXIT

# A native one-entry manifest must be selected explicitly and cannot masquerade
# as a complete five-platform candidate.
"$root/scripts/verify-release-manifest.sh" --native "$manifest" >/dev/null
if "$root/scripts/verify-release-manifest.sh" --aggregate "$manifest" >/dev/null 2>&1; then echo "partial aggregate manifest was accepted" >&2; exit 1; fi
if "$root/scripts/verify-release-manifest.sh" "$manifest" >/dev/null 2>&1; then echo "implicit manifest mode was accepted" >&2; exit 1; fi

# Merge rejects duplicate triples before it can emit an aggregate.
if node "$root/scripts/release-metadata.mjs" merge-manifests "$temp/duplicate.json" "$manifest" "$manifest" >/dev/null 2>&1; then echo "duplicate target triple was accepted" >&2; exit 1; fi

# Exactly five clean, uniquely named supported targets merge successfully.
targets=(aarch64-apple-darwin aarch64-unknown-linux-gnu x86_64-apple-darwin x86_64-pc-windows-msvc x86_64-unknown-linux-gnu)
inputs=()
for index in "${!targets[@]}"; do
  output="$temp/native-$index.json"; target=${targets[$index]}
  jq --arg target "$target" --arg archive "fixture-$index.tar.gz" '.source_dirty = false | .artifacts[0].target_triple = $target | .artifacts[0].archive_name = $archive' "$manifest" >"$output"
  inputs+=("$output")
done
node "$root/scripts/release-metadata.mjs" merge-manifests "$temp/exact-five.json" "${inputs[@]}"
jq -e '.source_dirty == false and (.artifacts | length) == 5' "$temp/exact-five.json" >/dev/null

# A link is not a regular release payload. Rebuild the otherwise-valid native
# archive with one symlink, update only the outer archive hash, and require the
# independent verifier to reject the entry type.
cp -R "$(dirname "$manifest")/." "$temp/dist/"
archive_name=$(jq -r '.artifacts[0].archive_name' "$temp/dist/release-manifest.json")
archive="$temp/dist/$archive_name"
mkdir "$temp/unpack"; tar -xzf "$archive" -C "$temp/unpack"
ln -s LICENSE "$temp/unpack/forbidden-link"
(cd "$temp/unpack" && tar -czf "$archive" .)
if command -v sha256sum >/dev/null 2>&1; then sha=$(sha256sum "$archive" | awk '{print $1}'); else sha=$(shasum -a 256 "$archive" | awk '{print $1}'); fi
jq --arg sha "$sha" '.artifacts[0].archive_sha256 = $sha' "$temp/dist/release-manifest.json" >"$temp/manifest.json" && mv "$temp/manifest.json" "$temp/dist/release-manifest.json"
printf '%s  %s\n' "$sha" "$archive_name" >"$archive.sha256"
if "$root/scripts/verify-release-manifest.sh" --native "$temp/dist/release-manifest.json" >/dev/null 2>&1; then echo "symlink archive entry was accepted" >&2; exit 1; fi

# Dirty release inputs fail closed unless the clearly test-only override is
# present; metadata truthfulness is checked by the reproducibility test.
touch "$dirty"
if env -u SYMPHONY_RELEASE_ALLOW_DIRTY_TEST_ONLY \
  SYMPHONY_RELEASE_OUTPUT="$temp/dirty-dist" \
  "$root/scripts/build-release.sh" >/dev/null 2>&1; then
  echo "dirty release inputs were accepted without override" >&2
  exit 1
fi
rm -f "$dirty"
echo "Release negative fixtures passed"
