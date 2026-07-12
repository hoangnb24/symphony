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

fixture="$temp/incompatible fixture"
mkdir -p "$fixture/.harness" "$fixture/scripts/schema"
db="$fixture/harness.db"
HARNESS_REPO_ROOT="$repo_root" HARNESS_DB_PATH="$db" "$cli" init >/dev/null
HARNESS_REPO_ROOT="$repo_root" HARNESS_DB_PATH="$db" "$cli" story add \
  --id US-NEGATIVE --title "Compatibility hash sentinel" --lane normal --verify true --json >/dev/null

old_cli="$fixture/old harness cli"
cat >"$old_cli" <<'SH'
#!/bin/sh
set -eu
printf '%s\n' '{"protocol_version":1,"operation":"query.contract","request_id":"negative","result":{"protocol_version":1,"cli_version":"0.1.11","schema_minimum":1,"schema_maximum":13,"database_state":"current","database_schema_version":13,"required_environment_variables":["HARNESS_DB_PATH"],"capabilities":["stories.read.v1","stories.write.v1","work-graph.read.v1","story-dependencies.read-write.v1","story-hierarchy.read-write.v1","changesets.apply.v1","changesets.status-sha.v1","isolated-db.v1","isolated-db-snapshot.v1","semantic-operation-log.v1"]}}'
SH
chmod +x "$old_cli"
cat >"$fixture/.harness/symphony.yml" <<YAML
version: 1
repo:
  root: .
  harness_db: harness.db
  harness_cli: old harness cli
YAML

before_json=$(HARNESS_REPO_ROOT="$repo_root" HARNESS_DB_PATH="$db" "$cli" db snapshot --output "$temp/before.db" --json)
before=$(jq -er '.result.source_logical_sha256' <<<"$before_json")
set +e
"$binary" --repo-root "$fixture" doctor >"$temp/old.stdout" 2>"$temp/old.stderr"
rc=$?
set -e
[[ $rc -ne 0 ]] || { echo "legacy CLI unexpectedly passed doctor" >&2; exit 1; }
rg -q '0\.1\.11' "$temp/old.stdout" "$temp/old.stderr"
rg -q 'harness-cli-v0\.1\.14' "$temp/old.stdout" "$temp/old.stderr"
[[ ! -e "$fixture/.symphony/state.db" ]] || {
  echo "incompatible preflight wrote Symphony state" >&2
  exit 1
}
after_json=$(HARNESS_REPO_ROOT="$repo_root" HARNESS_DB_PATH="$db" "$cli" db snapshot --output "$temp/after.db" --json)
after=$(jq -er '.result.source_logical_sha256' <<<"$after_json")
[[ "$before" == "$after" ]] || { echo "incompatible preflight changed Harness logical state" >&2; exit 1; }

"$repo_root/tests/architecture/no-direct-harness-db-access.sh"
HARNESS_CLI_PATH="$cli" "$repo_root/tests/compatibility/test-harness-wal-snapshot.sh"

echo "Harness protocol compatibility passed; negative logical hash remained $before"
