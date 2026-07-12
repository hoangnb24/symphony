#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cli=${HARNESS_CLI_PATH:-"$repo_root/scripts/bin/harness-cli"}
binary="$repo_root/target/debug/harness-symphony"
temp=$(mktemp -d)
trap 'rm -rf "$temp"' EXIT

[[ -x "$cli" ]] || { echo "missing Harness CLI: $cli" >&2; exit 1; }
[[ "$($cli --version)" == "harness-cli 0.1.14" ]] || {
  echo "compatibility suite requires the immutable harness-cli-v0.1.14 artifact" >&2
  exit 1
}

cargo build --manifest-path "$repo_root/Cargo.toml" -p harness-symphony --locked >/dev/null
cargo test --manifest-path "$repo_root/Cargo.toml" -p harness-symphony --locked \
  harness_protocol::tests -- --nocapture
cargo test --manifest-path "$repo_root/Cargo.toml" -p harness-symphony --locked \
  work::protocol_tests -- --nocapture
cargo test --manifest-path "$repo_root/Cargo.toml" -p harness-symphony --locked \
  doctor::tests -- --nocapture
cargo test --manifest-path "$repo_root/Cargo.toml" -p harness-symphony --locked \
  sync::tests -- --nocapture

full_capabilities='["stories.read.v1","stories.write.v1","work-graph.read.v1","story-dependencies.read-write.v1","story-hierarchy.read-write.v1","changesets.apply.v1","changesets.status-sha.v1","isolated-db.v1","isolated-db-snapshot.v1","semantic-operation-log.v1"]'

run_negative_fixture() {
  local name=$1
  local response=$2
  local expected=$3
  local fixture="$temp/$name fixture"
  local db="$fixture/harness.db"
  mkdir -p "$fixture/.harness" "$fixture/.harness/changesets"
  git -C "$fixture" init -q
  printf '%s\n' 'harness.db' 'harness.db-wal' 'harness.db-shm' '.symphony/' >"$fixture/.gitignore"
  HARNESS_REPO_ROOT="$repo_root" HARNESS_DB_PATH="$db" "$cli" init >/dev/null
  HARNESS_REPO_ROOT="$fixture" HARNESS_DB_PATH="$db" "$cli" story add \
    --id US-NEGATIVE --title "Compatibility hash sentinel" --lane normal --verify true --json >/dev/null
  printf '%s\n' '{"version":1,"id":"sentinel","operations":[]}' >"$fixture/.harness/changesets/sentinel.changeset.jsonl"

  local fake_cli="$fixture/fixture harness cli"
  cat >"$fake_cli" <<SH
#!/bin/sh
set -eu
printf '%s\n' '$response'
SH
  chmod +x "$fake_cli"
  cat >"$fixture/.harness/symphony.yml" <<YAML
version: 1
repo:
  root: .
  harness_db: harness.db
  harness_cli: fixture harness cli
YAML

  local before_json before after_json after changesets_before changesets_after
  before_json=$(HARNESS_REPO_ROOT="$fixture" HARNESS_DB_PATH="$db" "$cli" db snapshot --output "$temp/$name-before.db" --json)
  before=$(jq -er '.result.source_logical_sha256' <<<"$before_json")
  changesets_before=$(find "$fixture/.harness/changesets" -type f -print -exec shasum -a 256 {} \; | LC_ALL=C sort)
  set +e
  "$binary" --repo-root "$fixture" doctor >"$temp/$name.stdout" 2>"$temp/$name.stderr"
  local rc=$?
  set -e
  [[ $rc -ne 0 ]] || { echo "$name CLI unexpectedly passed doctor" >&2; exit 1; }
  rg -qi "$expected" "$temp/$name.stdout" "$temp/$name.stderr"
  [[ ! -e "$fixture/.symphony/state.db" ]] || {
    echo "$name preflight wrote Symphony state" >&2
    exit 1
  }
  after_json=$(HARNESS_REPO_ROOT="$fixture" HARNESS_DB_PATH="$db" "$cli" db snapshot --output "$temp/$name-after.db" --json)
  after=$(jq -er '.result.source_logical_sha256' <<<"$after_json")
  changesets_after=$(find "$fixture/.harness/changesets" -type f -print -exec shasum -a 256 {} \; | LC_ALL=C sort)
  [[ "$before" == "$after" ]] || { echo "$name preflight changed Harness logical state" >&2; exit 1; }
  [[ "$changesets_before" == "$changesets_after" ]] || { echo "$name preflight changed active changesets" >&2; exit 1; }
}

run_negative_fixture legacy \
  "{\"protocol_version\":1,\"operation\":\"query.contract\",\"request_id\":\"negative\",\"result\":{\"protocol_version\":1,\"cli_version\":\"0.1.11\",\"schema_minimum\":1,\"schema_maximum\":13,\"database_state\":\"current\",\"database_schema_version\":13,\"required_environment_variables\":[\"HARNESS_DB_PATH\"],\"capabilities\":$full_capabilities}}" \
  '0\.1\.11|harness-cli-v0\.1\.14'
run_negative_fixture malformed 'this is not protocol JSON' 'malformed JSON'
missing_capabilities=${full_capabilities/\"isolated-db-snapshot.v1\",/}
run_negative_fixture missing-capability \
  "{\"protocol_version\":1,\"operation\":\"query.contract\",\"request_id\":\"negative\",\"result\":{\"protocol_version\":1,\"cli_version\":\"0.1.14\",\"schema_minimum\":1,\"schema_maximum\":13,\"database_state\":\"current\",\"database_schema_version\":13,\"required_environment_variables\":[\"HARNESS_DB_PATH\"],\"capabilities\":$missing_capabilities}}" \
  'missing capability isolated-db-snapshot\.v1'

"$repo_root/tests/architecture/no-direct-harness-db-access.sh"
HARNESS_CLI_PATH="$cli" "$repo_root/tests/compatibility/test-harness-wal-snapshot.sh"

echo "Harness protocol compatibility passed; legacy, malformed, and missing-capability fixtures failed before mutation"
