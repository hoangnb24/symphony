#!/usr/bin/env bash
set -euo pipefail
fixture=$(cd "${1:?fixture required}" && pwd)
expected=${2:-harness-cli-v0.1.14}
cli="$fixture/scripts/bin/harness-cli"; [[ -x "$cli" ]] || cli="$fixture/scripts/bin/harness-cli.exe"
json=$(cd "$fixture" && "$cli" query contract --json)
jq -e --arg version "${expected#harness-cli-v}" '
  .result.protocol_version == 1 and .result.cli_version == $version and
  .result.schema_minimum == 1 and .result.schema_maximum == 13 and
  .result.database_state == "current" and .result.database_schema_version == 13 and
  (["stories.read.v1","stories.write.v1","work-graph.read.v1","story-dependencies.read-write.v1","story-hierarchy.read-write.v1","changesets.apply.v1","changesets.status-sha.v1","isolated-db.v1","isolated-db-snapshot.v1","semantic-operation-log.v1"] - .result.capabilities | length == 0)
' <<<"$json" >/dev/null
echo "Harness contract tuple accepted: $expected"
