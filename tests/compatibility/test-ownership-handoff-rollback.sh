#!/usr/bin/env bash
set -euo pipefail

target_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
source_root=${E11_SOURCE_ROOT:-/Users/themrb/Documents/personal/repository-harness}
cli=${HARNESS_CLI_PATH:-"$target_root/scripts/bin/harness-cli"}
temp=$(mktemp -d)
trap 'rm -rf "$temp"' EXIT

[[ -s "$source_root/harness.db" ]] || { echo "missing source Harness DB" >&2; exit 1; }
[[ -s "$target_root/harness.db" ]] || { echo "missing target Harness DB" >&2; exit 1; }

snapshot() {
  local root=$1 source=$2 output=$3
  HARNESS_REPO_ROOT="$root" HARNESS_DB_PATH="$source" \
    "$cli" db snapshot --output "$output" --json
}

source_before_json=$(snapshot "$source_root" "$source_root/harness.db" "$temp/source.db")
target_before_json=$(snapshot "$target_root" "$target_root/harness.db" "$temp/target.db")
source_before=$(jq -er '.result.source_logical_sha256' <<<"$source_before_json")
target_before=$(jq -er '.result.source_logical_sha256' <<<"$target_before_json")

query_graph() {
  local root=$1 db=$2
  HARNESS_REPO_ROOT="$root" HARNESS_DB_PATH="$db" "$cli" query work-graph --json
}

assert_owner() {
  local expected=$1
  local source_graph target_graph owners
  source_graph=$(query_graph "$source_root" "$temp/source.db")
  target_graph=$(query_graph "$target_root" "$temp/target.db")
  owners=$(jq -n \
    --argjson source "$source_graph" --argjson target "$target_graph" '
      [
        ($source.result.stories[] | select(.id=="US-093" and .runnable) | "source"),
        ($target.result.stories[] | select(.id=="US-093" and .runnable) | "target")
      ]')
  [[ "$(jq -r 'length' <<<"$owners")" == 1 ]]
  [[ "$(jq -r '.[0]' <<<"$owners")" == "$expected" ]]
}

assert_owner target

# Rehearse rollback ordering: disable target first, restore source second.
HARNESS_REPO_ROOT="$target_root" HARNESS_DB_PATH="$temp/target.db" \
  "$cli" story update --id US-093 --status changed --expected-status planned \
    --require-runnable --json >/dev/null
source_mid=$(query_graph "$source_root" "$temp/source.db")
target_mid=$(query_graph "$target_root" "$temp/target.db")
jq -e '[.result.stories[]|select(.id=="US-093" and .runnable)]|length==0' <<<"$source_mid" >/dev/null
jq -e '[.result.stories[]|select(.id=="US-093" and .runnable)]|length==0' <<<"$target_mid" >/dev/null
HARNESS_REPO_ROOT="$source_root" HARNESS_DB_PATH="$temp/source.db" \
  "$cli" story update --id US-093 --status planned --expected-status changed --json >/dev/null
assert_owner source

# Rehearse forward recovery ordering: disable source first, activate target second.
HARNESS_REPO_ROOT="$source_root" HARNESS_DB_PATH="$temp/source.db" \
  "$cli" story update --id US-093 --status changed --expected-status planned --json >/dev/null
HARNESS_REPO_ROOT="$target_root" HARNESS_DB_PATH="$temp/target.db" \
  "$cli" story update --id US-093 --status planned --expected-status changed --json >/dev/null
assert_owner target

source_after_json=$(snapshot "$source_root" "$source_root/harness.db" "$temp/source-after.db")
target_after_json=$(snapshot "$target_root" "$target_root/harness.db" "$temp/target-after.db")
[[ "$(jq -r '.result.source_logical_sha256' <<<"$source_after_json")" == "$source_before" ]]
[[ "$(jq -r '.result.source_logical_sha256' <<<"$target_after_json")" == "$target_before" ]]

echo "US-093 rollback rehearsal passed: target -> fenced zero -> source -> target; canonical hashes unchanged"
