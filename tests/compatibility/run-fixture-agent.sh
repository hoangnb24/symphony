#!/usr/bin/env bash
set -euo pipefail
: "${HARNESS_DB_PATH:?}"
: "${HARNESS_RUN_ID:?}"
: "${HARNESS_RUN_MODE:?}"
[[ "$HARNESS_RUN_MODE" == execute ]]
story=${1:-US-INDEP-001}
cli=${2:-"$PWD/scripts/bin/harness-cli"}
[[ -x "$cli" ]] || cli="$PWD/scripts/bin/harness-cli.exe"
[[ -x "$cli" ]] || { echo "fixture Harness CLI is missing" >&2; exit 1; }
"$cli" story verify "$story"
"$cli" story complete "$story" --json >"/tmp/${HARNESS_RUN_ID}-complete.json"
run_dir=".harness/runs/$HARNESS_RUN_ID"
mkdir -p "$run_dir"
printf '{"event":"fixture-agent.completed","run_id":"%s","story_id":"%s"}\n' "$HARNESS_RUN_ID" "$story" >"$run_dir/AGENT_EVENTS.jsonl"
cat >"$run_dir/SUMMARY.md" <<EOF
# Deterministic fixture result

Completed $story through the isolated Harness protocol.
EOF
jq -n --arg run "$HARNESS_RUN_ID" --arg story "$story" '{version:1,run_id:$run,story_id:$story,outcome:"completed",summary_path:(".harness/runs/"+$run+"/SUMMARY.md"),validation:{commands:[{command:"fixture deterministic validation",result:"pass"}]}}' >"$run_dir/RESULT.json"
