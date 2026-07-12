#!/usr/bin/env bash
set -euo pipefail
archive=${1:?archive required}; fixture=$(cd "${2:?fixture required}" && pwd)
root=$(cd "$(dirname "$0")/../.." && pwd)
temp=$(mktemp -d); pid=
cleanup() { [[ -z "$pid" ]] || kill "$pid" 2>/dev/null || true; rm -rf "$temp"; }
trap cleanup EXIT
tar -xzf "$archive" -C "$temp"
binary="$temp/bin/harness-symphony"
[[ -x "$binary" ]]
"$binary" --version | rg -q '^harness-symphony '
"$binary" version --json | jq -e '. == {symphony_version:"0.1.0",harness_protocol_version:1,harness_schema_minimum:1,harness_schema_maximum:13,current_harness_schema_minimum:12,current_harness_schema_maximum:13,supported_harness_cli_versions:["0.1.14"]}' >/dev/null
cli="$fixture/scripts/bin/harness-cli"; [[ -x "$cli" ]] || cli="$fixture/scripts/bin/harness-cli.exe"
contract=$(cd "$fixture" && "$cli" query contract --json)
jq -e '.protocol_version == 1 and .operation == "query.contract" and .result.cli_version == "0.1.14" and .result.protocol_version == 1 and .result.schema_minimum == 1 and .result.schema_maximum == 13 and .result.database_schema_version == 13 and .result.database_state == "current" and (["stories.read.v1","stories.write.v1","work-graph.read.v1","story-dependencies.read-write.v1","story-hierarchy.read-write.v1","changesets.apply.v1","changesets.status-sha.v1","isolated-db.v1","isolated-db-snapshot.v1","semantic-operation-log.v1"] - .result.capabilities | length == 0)' <<<"$contract" >/dev/null
graph=$(cd "$fixture" && "$cli" query work-graph --json)
jq -e '.protocol_version == 1 and .operation == "query.work-graph" and (.result.revision | type == "string") and (.result.stories | type == "array")' <<<"$graph" >/dev/null
caller="$temp/caller"; mkdir "$caller"; git -C "$caller" init -q
(cd "$caller" && "$binary" --repo-root "$fixture" doctor)
log="$temp/web.log"
(cd "$caller" && "$binary" --repo-root "$fixture" web --host 127.0.0.1 --port 0) >"$log" 2>&1 & pid=$!
base=
for _ in {1..300}; do base=$(sed -n 's/.*\(http:\/\/127\.0\.0\.1:[0-9][0-9]*\).*/\1/p' "$log" | tail -1); [[ -n "$base" ]] && break; kill -0 "$pid" 2>/dev/null || { cat "$log" >&2; exit 1; }; sleep 0.1; done
[[ -n "$base" ]]
curl -fsS "$base/health" | jq -e '.ok == true' >/dev/null
curl -fsS "$base/api/board" | jq -e '.items | type == "array"' >/dev/null
index=$(curl -fsS "$base/"); rg -q '<div id="root"></div>' <<<"$index"
while IFS= read -r asset; do curl -fsS "$base$asset" >/dev/null; done < <(sed -n "s#.*\(/assets/[^\"' ]*\).*#\1#p" <<<"$index" | sort -u)
echo "Release artifact smoke passed"
