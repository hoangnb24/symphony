#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
temp=$(mktemp -d)
trap 'rm -rf "$temp"' EXIT

[[ ${E11_US096_FORCE_FAILURE:-0} != 1 ]] || {
  echo "intentional US-096 negative verification fixture" >&2
  exit 1
}

cd "$root"
ruby -e 'require "yaml"; YAML.load_file(ARGV.fetch(0))' \
  .github/workflows/release-candidate.yml
rg -q '^  contents: read$' .github/workflows/release-candidate.yml
! rg -n 'contents: write|gh release|create-release|softprops/action-gh-release' \
  .github/workflows/release-candidate.yml
for label in linux-x64 linux-arm64 macos-arm64 macos-x64 windows-x64; do
  test "$(rg -c "label: $label" .github/workflows/release-candidate.yml)" = 1
done
rg -q 'ubuntu-24\.04-arm' .github/workflows/release-candidate.yml
rg -q 'macos-15-intel' .github/workflows/release-candidate.yml
rg -q 'windows-2025' .github/workflows/release-candidate.yml
rg -q 'merge-multiple: false' .github/workflows/release-candidate.yml
rg -q 'retention-days: 7' .github/workflows/release-candidate.yml
rg -q 'version --json' .github/workflows/release-candidate.yml
rg -q 'desktop:build' .github/workflows/release-candidate.yml
rg -q 'desktop:packaged-smoke' .github/workflows/release-candidate.yml
rg -q 'scripts/merge-release-manifests.sh' .github/workflows/release-candidate.yml
rg -q 'verify-release-manifest.sh --native' .github/workflows/release-candidate.yml
rg -q 'verify-release-manifest.sh --aggregate' .github/workflows/release-candidate.yml
rg -q 'CSC_IDENTITY_AUTO_DISCOVERY' .github/workflows/release-candidate.yml

cargo fmt --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --release --locked
version=$(target/release/harness-symphony version --json)
jq -e '
  .symphony_version == "0.1.0" and
  .harness_protocol_version == 1 and
  .harness_schema_minimum == 1 and
  .harness_schema_maximum == 13 and
  .current_harness_schema_minimum == 12 and
  .current_harness_schema_maximum == 13 and
  .supported_harness_cli_versions == ["0.1.14"]
' <<<"$version" >/dev/null
(cd "$temp" && "$root/target/release/harness-symphony" version --json) | cmp - <(printf '%s\n' "$version")

target=$(rustc -vV | sed -n 's/^host: //p')
case "$target" in
  aarch64-apple-darwin) label=macos-arm64 ;;
  x86_64-apple-darwin) label=macos-x64 ;;
  aarch64-unknown-linux-gnu) label=linux-arm64 ;;
  x86_64-unknown-linux-gnu) label=linux-x64 ;;
  *) echo "unsupported local release target: $target" >&2; exit 1 ;;
esac
export SYMPHONY_RELEASE_ALLOW_DIRTY_TEST_ONLY=1
cargo build --workspace --release --locked --target "$target"
native_binary="$root/target/$target/release/harness-symphony"
SOURCE_SHA=$(git rev-parse HEAD) \
SYMPHONY_RELEASE_TARGET="$target" \
SYMPHONY_RELEASE_LABEL="$label" \
SYMPHONY_RELEASE_OUTPUT="$temp/dist" \
SYMPHONY_BINARY="$native_binary" \
  scripts/build-release.sh >/dev/null
scripts/verify-release-manifest.sh --native "$temp/dist/release-manifest.json"
archive=$(find "$temp/dist" -maxdepth 1 -name '*.tar.gz' -print)
test "$(printf '%s\n' "$archive" | sed '/^$/d' | wc -l | tr -d ' ')" = 1
tests/release/test-artifact-boundary.sh "$archive"
tests/release/test-reproducible-package.sh
tests/release/test-release-negative-fixtures.sh "$temp/dist/release-manifest.json"
mkdir -p "$temp/unpack"
tar -xzf "$archive" -C "$temp/unpack"
packaged_version=$("$temp/unpack/bin/harness-symphony" version --json)
jq -e --argjson report "$packaged_version" '
  $report.symphony_version == .symphony_version and
  $report.harness_protocol_version == .supported_harness.protocol_version and
  $report.harness_schema_minimum == .supported_harness.schema_minimum and
  $report.harness_schema_maximum == .supported_harness.schema_maximum and
  $report.current_harness_schema_minimum == .supported_harness.current_schema_minimum and
  $report.current_harness_schema_maximum == .supported_harness.current_schema_maximum and
  $report.supported_harness_cli_versions == ["0.1.14"]
' "$temp/dist/release-manifest.json" >/dev/null
merge_inputs=()
for triple in \
  aarch64-apple-darwin \
  aarch64-unknown-linux-gnu \
  x86_64-apple-darwin \
  x86_64-pc-windows-msvc \
  x86_64-unknown-linux-gnu; do
  synthetic="$temp/dist/synthetic-$triple.json"
  jq --arg triple "$triple" \
    --arg archive "synthetic-$triple.tar.gz" \
    '.source_dirty = false | .artifacts[0].target_triple = $triple | .artifacts[0].archive_name = $archive' \
    "$temp/dist/release-manifest.json" >"$synthetic"
  merge_inputs+=("$synthetic")
done
scripts/merge-release-manifests.sh "$temp/dist/merged-manifest.json" \
  "${merge_inputs[@]}" >/dev/null
test "$(jq '.artifacts | length' "$temp/dist/merged-manifest.json")" = 5

fixture="$temp/fixture"
tests/compatibility/bootstrap-harness-fixture.sh \
  --upgrade-cli --story US-RELEASE-CANDIDATE "$fixture" >/dev/null
tests/compatibility/smoke-release-artifact.sh "$archive" "$fixture"
CSC_IDENTITY_AUTO_DISCOVERY=false \
  npm --prefix crates/harness-symphony/web-ui run desktop:build
npm --prefix crates/harness-symphony/web-ui run desktop:packaged-smoke -- \
  --repo-root "$fixture"

git diff --check
echo "US-096 release-candidate CI/version verification passed without publication"
