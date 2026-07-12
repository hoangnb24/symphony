#!/usr/bin/env bash
set -euo pipefail
mode=${1:-}; shift || true
[[ "$mode" == --native || "$mode" == --aggregate ]] || { echo "usage: $0 --native|--aggregate MANIFEST" >&2; exit 2; }
manifest=$(cd "$(dirname "${1:?manifest required}")" && pwd)/$(basename "$1")
dist=$(dirname "$manifest")
root=$(cd "$(dirname "$0")/.." && pwd)
hash_file() { if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}'; else shasum -a 256 "$1" | awk '{print $1}'; fi; }
jq -e '.manifest_version == 1 and .product == "harness-symphony" and (.source_dirty | type == "boolean") and (.artifacts | length > 0) and .supported_harness == {protocol_version:1,schema_minimum:1,schema_maximum:13,current_schema_minimum:12,current_schema_maximum:13}' "$manifest" >/dev/null
expected_triples='["aarch64-apple-darwin","aarch64-unknown-linux-gnu","x86_64-apple-darwin","x86_64-pc-windows-msvc","x86_64-unknown-linux-gnu"]'
if [[ "$mode" == --native ]]; then
  jq -e --argjson expected "$expected_triples" '(.artifacts | length) == 1 and (.artifacts[0].target_triple as $triple | $expected | index($triple)) != null' "$manifest" >/dev/null
else
  jq -e --argjson expected "$expected_triples" '.source_dirty == false and ([.artifacts[].target_triple] | sort) == $expected and (.artifacts | length) == 5' "$manifest" >/dev/null
fi

temp=$(mktemp -d); trap 'rm -rf "$temp"' EXIT
count=$(jq '.artifacts | length' "$manifest")
for ((index=0; index<count; index++)); do
  entry=$(jq -c ".artifacts[$index]" "$manifest")
  archive_name=$(jq -r '.archive_name' <<<"$entry")
  [[ "$archive_name" != */* && "$archive_name" != *..* ]]
  archive="$dist/$archive_name"; [[ -s "$archive" ]]
  actual=$(hash_file "$archive"); expected=$(jq -r '.archive_sha256' <<<"$entry")
  [[ "$actual" == "$expected" ]]
  [[ "$(awk '{print $1}' "$archive.sha256")" == "$actual" ]]
  [[ "$(awk '{print $2}' "$archive.sha256")" == "$archive_name" ]]
  list="$temp/list-$index"
  tar -tf "$archive" >"$list"
  [[ $(LC_ALL=C sort "$list" | uniq -d | wc -l | tr -d ' ') == 0 ]]
  if awk 'BEGIN{bad=0} /^\//{bad=1} /(^|\/)\.\.($|\/)/{bad=1} END{exit bad?0:1}' "$list"; then echo "unsafe archive path" >&2; exit 1; fi
  if sed 's#^\./##' "$list" | rg '(^|/)(/|$)'; then echo "archive path contains an empty segment" >&2; exit 1; fi
  if tar -tvf "$archive" | awk 'substr($0,1,1) != "-" {bad=1} END{exit bad?0:1}'; then echo "archive contains a link or special entry" >&2; exit 1; fi
  rg -q '^\./bin/harness-symphony(\.exe)?$' "$list"
  rg -q '^\./share/harness-symphony/web-ui/' "$list"
  for required in LICENSE release-metadata.json provenance.json sbom.spdx.json share/harness-symphony/resource-manifest.json; do rg -q "^\\./$required$" "$list"; done
  if rg -i '(^|/)(harness\.db|harness\.db-wal|harness\.db-shm|harness-cli(\.exe)?|crates/harness-cli|\.symphony)(/|$)' "$list"; then echo "forbidden release content" >&2; exit 1; fi
  unpack="$temp/unpack-$index"; mkdir -p "$unpack"; tar -xzf "$archive" -C "$unpack"
  metadata="$unpack/release-metadata.json"; provenance="$unpack/provenance.json"; sbom="$unpack/sbom.spdx.json"
  [[ "$(hash_file "$metadata")" == "$(jq -r '.metadata_sha256' <<<"$entry")" ]]
  [[ "$(hash_file "$provenance")" == "$(jq -r '.provenance_sha256' <<<"$entry")" ]]
  [[ "$(hash_file "$sbom")" == "$(jq -r '.sbom_sha256' <<<"$entry")" ]]
  jq -e --argjson entry "$entry" --arg version "$(jq -r '.symphony_version' "$manifest")" --arg source "$(jq -r '.source_sha' "$manifest")" --argjson dirty "$(jq '.source_dirty' "$manifest")" '.symphony_version == $version and .source_sha == $source and .source_dirty == $dirty and .target_triple == $entry.target_triple and .binary_path == $entry.binary_path and .web_asset_root == $entry.web_asset_root and .web_asset_sha256 == $entry.web_asset_sha256' "$metadata" >/dev/null
  resource="$unpack/share/harness-symphony/resource-manifest.json"
  jq -e --arg binary "$(jq -r '.binary_path' <<<"$entry")" --arg web "$(jq -r '.web_asset_root' <<<"$entry")" --arg sha "$(jq -r '.web_asset_sha256' <<<"$entry")" '. == {format_version:1,binary_path:$binary,web_asset_root:$web,web_asset_sha256:$sha}' "$resource" >/dev/null
  [[ "$(node "$root/scripts/release-metadata.mjs" tree-hash "$unpack/$(jq -r '.web_asset_root' <<<"$entry")")" == "$(jq -r '.web_asset_sha256' <<<"$entry")" ]]
  jq -e '.spdxVersion == "SPDX-2.3" and (.files | length > 0)' "$sbom" >/dev/null
  jq -e --argjson dirty "$(jq '.source_dirty' "$manifest")" '.provenance_version == 1 and .source_dirty == $dirty and .publication == "local-or-ci-only" and .signing == "deferred"' "$provenance" >/dev/null
done
[[ $(jq -r '.artifacts[].archive_name' "$manifest" | sort | uniq -d | wc -l | tr -d ' ') == 0 ]]
echo "Release manifest verified: $count artifact(s)"
