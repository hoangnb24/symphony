#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
temp=$(mktemp -d)
dirty_fixture="$root/crates/harness-symphony/.release-dirty-fixture-$$"
trap 'rm -rf "$temp"; rm -f "$dirty_fixture"' EXIT
target=$(rustc -vV | sed -n 's/^host: //p')
binary="$root/target/$target/release/harness-symphony"
[[ -x "$binary" ]] || cargo build --manifest-path "$root/Cargo.toml" --release --locked --target "$target"
touch "$dirty_fixture"
DIST_DIR="$temp/one" SYMPHONY_BINARY="$binary" SYMPHONY_RELEASE_ALLOW_DIRTY_TEST_ONLY=1 "$root/scripts/build-release.sh" >/dev/null
DIST_DIR="$temp/two" SYMPHONY_BINARY="$binary" SYMPHONY_RELEASE_ALLOW_DIRTY_TEST_ONLY=1 "$root/scripts/build-release.sh" >/dev/null
one=$(find "$temp/one" -maxdepth 1 -name '*.tar.gz' -print); two=$(find "$temp/two" -maxdepth 1 -name '*.tar.gz' -print)
cmp "$one" "$two"
[[ "$(awk '{print $1}' "$one.sha256")" == "$(awk '{print $1}' "$two.sha256")" ]]
"$root/scripts/verify-release-manifest.sh" --native "$temp/one/release-manifest.json"
jq -e '.source_dirty == true' "$temp/one/release-manifest.json" >/dev/null
"$root/tests/release/test-artifact-boundary.sh" "$one"
echo "Reproducible native package passed"
