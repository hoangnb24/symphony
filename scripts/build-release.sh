#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
dist=${SYMPHONY_RELEASE_OUTPUT:-${DIST_DIR:-"$root/dist"}}
target=${SYMPHONY_RELEASE_TARGET:-${TARGET_TRIPLE:-$(rustc -vV | sed -n 's/^host: //p')}}
label=${SYMPHONY_RELEASE_LABEL:-$target}
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$root/crates/harness-symphony/Cargo.toml" | head -1)
source_sha=${SOURCE_SHA:-$(git -C "$root" rev-parse HEAD)}
head_sha=$(git -C "$root" rev-parse HEAD)
[[ "$source_sha" == "$head_sha" ]] || { echo "SOURCE_SHA must equal checked-out HEAD" >&2; exit 1; }
release_status=$(git -C "$root" status --porcelain --untracked-files=all -- LICENSE Cargo.toml Cargo.lock rust-toolchain.toml .node-version crates/harness-symphony scripts/build-release.sh scripts/release-metadata.mjs scripts/verify-release-manifest.sh)
source_dirty=false
if [[ -n "$release_status" ]]; then
  [[ ${SYMPHONY_RELEASE_ALLOW_DIRTY_TEST_ONLY:-0} == 1 ]] || {
    echo "release inputs are dirty; commit them before packaging:" >&2; printf '%s\n' "$release_status" >&2; exit 1;
  }
  source_dirty=true
  echo "warning: test-only dirty release override active" >&2
fi
epoch=${SOURCE_DATE_EPOCH:-$(git -C "$root" show -s --format=%ct "$source_sha")}
binary_name=harness-symphony; [[ "$target" == *windows* ]] && binary_name=harness-symphony.exe
if [[ -n ${SYMPHONY_BINARY:-} ]]; then
  binary=$SYMPHONY_BINARY
else
  binary="$root/target/$target/release/$binary_name"
  cargo build --manifest-path "$root/Cargo.toml" --release --locked --target "$target"
fi
[[ -x "$binary" || "$binary_name" == *.exe && -f "$binary" ]] || { echo "release binary missing: $binary" >&2; exit 1; }
"$binary" version --json | jq -e --arg version "$version" '.symphony_version == $version and .harness_protocol_version == 1 and .harness_schema_minimum == 1 and .harness_schema_maximum == 13 and .current_harness_schema_minimum == 12 and .current_harness_schema_maximum == 13 and .supported_harness_cli_versions == ["0.1.14"]' >/dev/null
npm --prefix "$root/crates/harness-symphony/web-ui" run build

mkdir -p "$dist"
work=$(mktemp -d); trap 'rm -rf "$work"' EXIT
stage="$work/stage"
mkdir -p "$stage/bin" "$stage/share/harness-symphony/web-ui"
cp "$binary" "$stage/bin/$binary_name"
cp -R "$root/crates/harness-symphony/web-ui/dist/." "$stage/share/harness-symphony/web-ui/"
cp "$root/LICENSE" "$stage/LICENSE"
web_sha=$(node "$root/scripts/release-metadata.mjs" tree-hash "$stage/share/harness-symphony/web-ui")
jq -n --arg binary "bin/$binary_name" --arg web "share/harness-symphony/web-ui" --arg sha "$web_sha" '{format_version:1,binary_path:$binary,web_asset_root:$web,web_asset_sha256:$sha}' >"$stage/share/harness-symphony/resource-manifest.json"
node "$root/scripts/release-metadata.mjs" generate "$stage" "$stage/release-metadata.json" "$version" "$source_sha" "$target" "bin/$binary_name" "$epoch" "$(rustc --version)" "$(node --version)" "$source_dirty"
jq -n --arg source "$source_sha" --arg target "$target" --argjson epoch "$epoch" --argjson dirty "$source_dirty" '{provenance_version:1,subject:"harness-symphony release candidate",source_sha:$source,source_dirty:$dirty,target_triple:$target,source_date_epoch:$epoch,builder:{script:"scripts/build-release.sh"},materials:["Cargo.lock","crates/harness-symphony/web-ui/package-lock.json"],publication:"local-or-ci-only",signing:"deferred"}' >"$stage/provenance.json"
node "$root/scripts/release-metadata.mjs" sbom "$stage" "$stage/sbom.spdx.json" "$version" "$source_sha"

find "$stage" -type d -exec chmod 755 {} +
find "$stage" -type f -exec chmod 644 {} +
chmod 755 "$stage/bin/$binary_name"
touch_stamp=$(date -u -r "$epoch" +%Y%m%d%H%M.%S 2>/dev/null || date -u -d "@$epoch" +%Y%m%d%H%M.%S)
find "$stage" -exec touch -t "$touch_stamp" {} +

base="harness-symphony-$version-$label"
archive="$dist/$base.tar.gz"
(cd "$stage" && find . -type f -print | LC_ALL=C sort >"$work/files")
if tar --version 2>/dev/null | head -1 | grep -q 'GNU tar'; then
  tar_ownership=(--owner=0 --group=0 --numeric-owner)
else
  tar_ownership=(--uid 0 --gid 0 --uname root --gname root)
fi
COPYFILE_DISABLE=1 tar --format ustar "${tar_ownership[@]}" -C "$stage" -cf - -T "$work/files" | gzip -n -9 >"$archive"
hash_file() { if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}'; else shasum -a 256 "$1" | awk '{print $1}'; fi; }
archive_sha=$(hash_file "$archive")
printf '%s  %s\n' "$archive_sha" "$(basename "$archive")" >"$archive.sha256"
for file in release-metadata.json provenance.json sbom.spdx.json; do tar -xOf "$archive" "./$file" >"$work/$file"; done
node "$root/scripts/release-metadata.mjs" manifest "$dist/release-manifest.json" "$work/release-metadata.json" "$archive" "$archive_sha" tar.gz "$(hash_file "$work/release-metadata.json")" "$(hash_file "$work/provenance.json")" "$(hash_file "$work/sbom.spdx.json")"
printf '%s\n' "$archive"
