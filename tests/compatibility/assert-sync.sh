#!/usr/bin/env bash
set -euo pipefail
expect=""; unchanged=0
while [[ $# -gt 3 ]]; do
  case "$1" in --expect) expect=$2; shift 2;; --assert-state-unchanged) unchanged=1; shift;; *) exit 2;; esac
done
[[ $# -eq 3 && -n "$expect" ]] || { echo "usage: $0 --expect applied|no-op [--assert-state-unchanged] BIN FIXTURE RUN_ID" >&2; exit 2; }
bin=$1; fixture=$(cd "$2" && pwd); run_id=$3
cli="$fixture/scripts/bin/harness-cli"; [[ -x "$cli" ]] || cli="$fixture/scripts/bin/harness-cli.exe"
temp=$(mktemp -d); trap 'rm -rf "$temp"' EXIT
before=$((cd "$fixture" && "$cli" db snapshot --output "$temp/before.db" --json) | jq -r '.result.source_logical_sha256')
changeset="$fixture/.harness/changesets/$run_id.changeset.jsonl"
before_set=$((cd "$fixture" && "$cli" db changeset status --json "$changeset") | jq -r '.result.content_sha256')
output=$(cd "$(mktemp -d)" && "$bin" --repo-root "$fixture" sync)
after=$((cd "$fixture" && "$cli" db snapshot --output "$temp/after.db" --json) | jq -r '.result.source_logical_sha256')
after_set=$((cd "$fixture" && "$cli" db changeset status --json "$changeset") | jq -r '.result.content_sha256')
[[ "$before_set" == "$after_set" ]]
case "$expect" in
  applied) rg -q "$run_id applied \(3 operation\(s\)\)" <<<"$output"; [[ "$before" != "$after" ]] ;;
  no-op) rg -q "$run_id applied \(0 operation\(s\)\)" <<<"$output"; [[ "$before" == "$after" ]] ;;
  *) exit 2;;
esac
[[ $unchanged -eq 0 || "$before" == "$after" ]]
printf '%s\n' "$output"
printf 'logical_before=%s logical_after=%s changeset_set=%s\n' "$before" "$after" "$after_set"
