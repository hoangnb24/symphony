#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cli=${HARNESS_CLI_PATH:-"$repo_root/scripts/bin/harness-cli"}
db="$repo_root/harness.db"

[[ ${E11_US093_FORCE_FAILURE:-0} != 1 ]] || {
  echo "intentional US-093 negative verification fixture" >&2
  exit 1
}

[[ -x "$cli" ]] || { echo "missing executable Harness CLI: $cli" >&2; exit 1; }
[[ "$($cli --version)" == "harness-cli 0.1.14" ]] || {
  echo "US-093 requires harness-cli-v0.1.14" >&2
  exit 1
}
[[ -s "$db" ]] || { echo "missing initialized target Harness DB: $db" >&2; exit 1; }
[[ -s "$repo_root/.symphony/state.db" ]] || {
  echo "missing Symphony state DB and durable migration-fence record" >&2
  exit 1
}
[[ "$(sqlite3 "$repo_root/.symphony/state.db" 'SELECT COUNT(*) FROM migration_fence WHERE singleton=1;')" == 1 ]] || {
  echo "durable singleton migration fence was not initialized" >&2
  exit 1
}

awk '!/^#/ && NF {print $1 "  " $3}' "$repo_root/docs/provenance/e11-contract-packets.sha256" \
  | (cd "$repo_root" && shasum -a 256 -c -)

contract=$(HARNESS_REPO_ROOT="$repo_root" HARNESS_DB_PATH="$db" "$cli" query contract --json)
jq -e '
  .result.cli_version == "0.1.14" and
  .result.protocol_version == 1 and
  .result.schema_minimum == 1 and
  .result.schema_maximum == 13 and
  .result.database_state == "current" and
  .result.database_schema_version == 13 and
  .result.required_environment_variables == ["HARNESS_DB_PATH"] and
  ([.result.capabilities[]] | sort) == ([
    "changesets.apply.v1",
    "changesets.status-sha.v1",
    "isolated-db-snapshot.v1",
    "isolated-db.v1",
    "semantic-operation-log.v1",
    "stories.read.v1",
    "stories.write.v1",
    "story-dependencies.read-write.v1",
    "story-hierarchy.read-write.v1",
    "work-graph.read.v1"
  ] | sort)
' <<<"$contract" >/dev/null

graph=$(HARNESS_REPO_ROOT="$repo_root" HARNESS_DB_PATH="$db" "$cli" query work-graph --json)
jq -e '
  [.result.stories[] | select(.id | test("^US-09[3-6]$"))] as $stories |
  ($stories | length) == 4 and
  ($stories | all(.status == "planned")) and
  ($stories | map(select(.id != "US-093")) | all(.verify_command == null and .runnable == false)) and
  ([.result.dependencies[] | select((.blocker | test("^US-09[3-6]$")) or (.blocked | test("^US-09[3-6]$")))] | sort_by(.blocker,.blocked)) == ([
    {"blocker":"US-093","blocked":"US-094"},
    {"blocker":"US-094","blocked":"US-095"},
    {"blocker":"US-095","blocked":"US-096"}
  ] | sort_by(.blocker,.blocked))
' <<<"$graph" >/dev/null

cargo fmt --manifest-path "$repo_root/Cargo.toml" --all --check
cargo test --manifest-path "$repo_root/Cargo.toml" -p harness-symphony --locked
cargo clippy --manifest-path "$repo_root/Cargo.toml" -p harness-symphony --all-targets -- -D warnings
"$repo_root/tests/compatibility/test-harness-protocol.sh"
git -C "$repo_root" diff --check

echo "US-093 target adapter verification passed"
