#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cli=${HARNESS_CLI_PATH:-"$repo_root/scripts/bin/harness-cli"}
temp=$(mktemp -d)
reader_pid=
cleanup() {
  exec 3>&- 2>/dev/null || true
  [[ -z "$reader_pid" ]] || kill "$reader_pid" 2>/dev/null || true
  rm -rf "$temp"
}
trap cleanup EXIT

[[ -x "$cli" ]] || { echo "missing executable Harness CLI: $cli" >&2; exit 1; }
[[ "$($cli --version)" == "harness-cli 0.1.14" ]] || {
  echo "WAL snapshot compatibility is pinned to harness-cli 0.1.14" >&2
  exit 1
}

db="$temp/source.db"
snapshot="$temp/snapshot.db"
snapshot_two="$temp/snapshot-two.db"
export HARNESS_REPO_ROOT="$repo_root" HARNESS_DB_PATH="$db"
"$cli" init >/dev/null
sqlite3 "$db" 'PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;' >/dev/null

fifo="$temp/reader.fifo"
mkfifo "$fifo"
sqlite3 "$db" <"$fifo" >/dev/null &
reader_pid=$!
exec 3>"$fifo"
printf 'BEGIN; SELECT COUNT(*) FROM story;\n' >&3
sleep 0.1

"$cli" story add --id US-WAL --title "Uncheckpointed WAL story" --lane normal --verify true --json >/dev/null
[[ -s "$db-wal" ]] || { echo "fixture failed to retain an uncheckpointed WAL" >&2; exit 1; }

first=$("$cli" db snapshot --output "$snapshot" --json)
[[ -s "$snapshot" ]]
source_hash=$(jq -er '.result.source_logical_sha256' <<<"$first")
graph_revision=$(jq -er '.result.graph_revision' <<<"$first")

HARNESS_DB_PATH="$snapshot" "$cli" query work-graph --json >"$temp/snapshot-graph.json"
jq -e '.result.stories | any(.id == "US-WAL")' "$temp/snapshot-graph.json" >/dev/null
[[ "$(jq -r '.result.revision' "$temp/snapshot-graph.json")" == "$graph_revision" ]]

second=$("$cli" db snapshot --output "$snapshot_two" --json)
[[ "$(jq -r '.result.source_logical_sha256' <<<"$second")" == "$source_hash" ]]

echo "Harness WAL snapshot passed: $source_hash ($graph_revision)"
