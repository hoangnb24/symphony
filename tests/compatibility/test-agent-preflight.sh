#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cli=${HARNESS_CLI_PATH:-"$repo_root/scripts/bin/harness-cli"}
binary="$repo_root/target/debug/harness-symphony"
temp=$(mktemp -d)
trap 'rm -rf "$temp"' EXIT

fixture="$temp/missing agent fixture"
mkdir -p "$fixture/.harness/changesets" "$fixture/scripts/bin"
git -C "$fixture" init -q
git -C "$fixture" config user.name "Symphony Fixture"
git -C "$fixture" config user.email "symphony-fixture@example.invalid"
printf '%s\n' 'harness.db' 'harness.db-wal' 'harness.db-shm' '.symphony/' >"$fixture/.gitignore"
cp "$cli" "$fixture/scripts/bin/harness-cli"
chmod +x "$fixture/scripts/bin/harness-cli"
cat >"$fixture/.harness/symphony.yml" <<'YAML'
version: 1
repo:
  root: .
  harness_db: harness.db
  harness_cli: scripts/bin/harness-cli
agent:
  adapter: custom
  command: [symphony-agent-that-does-not-exist]
pull_request:
  create: disabled
YAML
HARNESS_REPO_ROOT="$repo_root" HARNESS_DB_PATH="$fixture/harness.db" "$cli" init >/dev/null
HARNESS_REPO_ROOT="$fixture" HARNESS_DB_PATH="$fixture/harness.db" "$cli" story add \
  --id US-MISSING-AGENT --title "Missing selected execution agent" --lane normal \
  --verify true --json >/dev/null
git -C "$fixture" add .
git -C "$fixture" commit -qm "fixture"

before=$(HARNESS_REPO_ROOT="$fixture" HARNESS_DB_PATH="$fixture/harness.db" "$cli" db snapshot \
  --output "$temp/before.db" --json | jq -er '.result.source_logical_sha256')
head_before=$(git -C "$fixture" rev-parse HEAD)
branches_before=$(git -C "$fixture" for-each-ref --format='%(refname)' refs/heads | LC_ALL=C sort)
changesets_before=$(find "$fixture/.harness/changesets" -type f -print -exec shasum -a 256 {} \; | LC_ALL=C sort)

set +e
"$binary" --repo-root "$fixture" run US-MISSING-AGENT >"$temp/stdout" 2>"$temp/stderr"
rc=$?
set -e
[[ $rc -ne 0 ]] || { echo "missing selected agent unexpectedly launched" >&2; exit 1; }
rg -q "selected agent executable 'symphony-agent-that-does-not-exist' is not available" \
  "$temp/stdout" "$temp/stderr"

after=$(HARNESS_REPO_ROOT="$fixture" HARNESS_DB_PATH="$fixture/harness.db" "$cli" db snapshot \
  --output "$temp/after.db" --json | jq -er '.result.source_logical_sha256')
branches_after=$(git -C "$fixture" for-each-ref --format='%(refname)' refs/heads | LC_ALL=C sort)
changesets_after=$(find "$fixture/.harness/changesets" -type f -print -exec shasum -a 256 {} \; | LC_ALL=C sort)
[[ "$before" == "$after" ]]
[[ "$head_before" == "$(git -C "$fixture" rev-parse HEAD)" ]]
[[ "$branches_before" == "$branches_after" ]]
[[ "$changesets_before" == "$changesets_after" ]]
[[ ! -e "$fixture/.symphony" ]]
[[ -z "$(git -C "$fixture" worktree list --porcelain | rg '^worktree ' | tail -n +2)" ]]

echo "Missing selected execution agent failed before branch, worktree, DB, changeset, or Symphony state mutation"
