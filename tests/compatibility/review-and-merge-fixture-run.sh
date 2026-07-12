#!/usr/bin/env bash
set -euo pipefail
fixture=$(cd "${1:?fixture required}" && pwd)
run_id=${2:?run id required}
worktree="$fixture/.symphony/worktrees/$run_id"
changeset="$worktree/.harness/changesets/$run_id.changeset.jsonl"
[[ -s "$changeset" ]]
[[ $(find "$worktree/.harness/changesets" -type f -name '*.changeset.jsonl' | wc -l | tr -d ' ') == 1 ]]
jq -s -e --arg run "$run_id" '
  length == 4 and
  .[0].op == "changeset.header" and .[0].version == 1 and .[0].run_id == $run and
  .[1].op == "story.update" and .[1].id == "US-INDEP-001" and .[1].payload.status == "in_progress" and
  .[2].op == "story.verify" and .[2].id == "US-INDEP-001" and .[2].payload.result == "pass" and
  .[3].op == "story.complete" and .[3].id == "US-INDEP-001" and .[3].payload.result == "pass"
' "$changeset" >/dev/null
git -C "$worktree" add ".harness/changesets/$run_id.changeset.jsonl"
git -C "$worktree" commit -q -m "test: review $run_id semantic changeset"
branch=$(git -C "$worktree" branch --show-current)
git -C "$fixture" merge -q --no-ff "$branch" -m "test: merge reviewed $run_id"
git -C "$fixture" rev-parse HEAD
