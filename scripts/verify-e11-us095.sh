#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
bin=${SYMPHONY_BIN:-"$root/target/release/harness-symphony"}
tag=harness-cli-v0.1.14
temp=$(mktemp -d)
reader_pid=
cleanup() {
  exec 3>&- 2>/dev/null || true
  [[ -z "$reader_pid" ]] || kill "$reader_pid" 2>/dev/null || true
  rm -rf "$temp"
}
trap cleanup EXIT

[[ ${E11_US095_FORCE_FAILURE:-0} != 1 ]] || {
  echo "intentional US-095 negative verification fixture" >&2
  exit 1
}

jq -e '
  .version == 1 and
  .story_id == "US-095" and
  .validation_mode == "reproducible-verifier" and
  .harness_release == "harness-cli-v0.1.14" and
  .forced_upgrade_from == "harness-cli-v0.1.11" and
  .expected_changeset_operations == ["story.update", "story.verify", "story.complete"] and
  .expected_first_sync_operations == 3 and
  .expected_second_sync_operations == 0 and
  .rust_tests == 119 and .playwright_tests == 19 and
  .verifier == "scripts/verify-e11-us095.sh"
' "$root/docs/provenance/e11-us095-parity.json" >/dev/null

if [[ ${E11_US095_SKIP_BASE_VALIDATION:-0} != 1 ]]; then
  cargo fmt --manifest-path "$root/Cargo.toml" --all -- --check
  cargo test --manifest-path "$root/Cargo.toml" --workspace --locked
  cargo clippy --manifest-path "$root/Cargo.toml" --workspace --all-targets -- -D warnings
  "$root/tests/compatibility/test-harness-protocol.sh"
  "$root/tests/compatibility/test-agent-preflight.sh"
fi
# `doctor` probes Git worktree support from the caller's directory. Make the
# deliberately third-party caller a Git repository without putting either
# product source tree on its runtime path.
git -C "$temp" init -q

cargo build --manifest-path "$root/Cargo.toml" --release --locked

prepare_fixture="$temp/prepare-fixture"
"$root/tests/compatibility/bootstrap-harness-fixture.sh" --upgrade-cli --story US-INDEP-001 "$prepare_fixture"
"$root/tests/compatibility/assert-contract-tuple.sh" "$prepare_fixture" "$tag"
cli="$prepare_fixture/scripts/bin/harness-cli"; [[ -x "$cli" ]] || cli="$prepare_fixture/scripts/bin/harness-cli.exe"
before=$((cd "$prepare_fixture" && "$cli" db snapshot --output "$temp/root-before.db" --json) | jq -r '.result.source_logical_sha256')
head_before=$(git -C "$prepare_fixture" rev-parse HEAD)
tracked_before=$(git -C "$prepare_fixture" status --porcelain --untracked-files=all)
ignored_before=$(git -C "$prepare_fixture" status --porcelain --ignored --untracked-files=all | LC_ALL=C sort)
changeset_before=$(if [[ -d "$prepare_fixture/.harness/changesets" ]]; then find "$prepare_fixture/.harness/changesets" -type f -name '*.changeset.jsonl' -print | LC_ALL=C sort; fi)
doctor_clean=$(cd "$temp" && "$bin" --repo-root "$prepare_fixture" doctor)
rg -q '\[PASS\] optional providers - no optional providers are registered; clean skip' <<<"$doctor_clean"
printf '%s\n' "$doctor_clean"
(cd "$temp" && "$bin" --repo-root "$prepare_fixture" work list | rg 'US-INDEP-001')
prepared=$(cd "$temp" && "$bin" --repo-root "$prepare_fixture" run US-INDEP-001 --prepare-only)
after=$((cd "$prepare_fixture" && "$cli" db snapshot --output "$temp/root-after.db" --json) | jq -r '.result.source_logical_sha256')
[[ "$before" == "$after" ]]
[[ "$(git -C "$prepare_fixture" rev-parse HEAD)" == "$head_before" ]]
[[ "$(git -C "$prepare_fixture" status --porcelain --untracked-files=all)" == "$tracked_before" ]]
changeset_after=$(if [[ -d "$prepare_fixture/.harness/changesets" ]]; then find "$prepare_fixture/.harness/changesets" -type f -name '*.changeset.jsonl' -print | LC_ALL=C sort; fi)
[[ "$changeset_before" == "$changeset_after" ]]
ignored_after=$(git -C "$prepare_fixture" status --porcelain --ignored --untracked-files=all | LC_ALL=C sort)
new_ignored=$(comm -13 <(printf '%s\n' "$ignored_before") <(printf '%s\n' "$ignored_after"))
while IFS= read -r path; do
  [[ -z "$path" || "$path" == "!! .harness/runs/"* || "$path" == "!! .symphony/"* ]] || {
    echo "prepare-only created unexpected local state: $path" >&2; exit 1;
  }
done <<<"$new_ignored"
prepare_run=$(sed -n 's/^Prepared run //p' <<<"$prepared")
isolated="$prepare_fixture/.symphony/worktrees/$prepare_run/harness.db"
sqlite3 "$isolated" 'PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;' >/dev/null
fifo="$temp/wal-reader.fifo"; mkfifo "$fifo"
sqlite3 "$isolated" <"$fifo" >/dev/null & reader_pid=$!
exec 3>"$fifo"
printf 'BEGIN; SELECT COUNT(*) FROM trace;\n' >&3
sleep 0.1
wal_summary="US-095 exact uncheckpointed WAL trace"
HARNESS_DB_PATH="$isolated" "$cli" trace --summary "$wal_summary" --outcome completed >/dev/null
[[ -s "$isolated-wal" ]] || { echo "isolated WAL was checkpointed unexpectedly" >&2; exit 1; }
snapshot=$(HARNESS_DB_PATH="$isolated" "$cli" db snapshot --output "$temp/wal-snapshot.db" --json)
jq -e '.result.source_logical_sha256 | length == 64' <<<"$snapshot" >/dev/null
HARNESS_DB_PATH="$temp/wal-snapshot.db" "$cli" query sql "SELECT task_summary FROM trace WHERE task_summary = '$wal_summary';" | rg -Fq "$wal_summary"
exec 3>&-; wait "$reader_pid"; reader_pid=

# Registered-but-missing optional providers weaken proof but do not block the
# same doctor/work/prepare operator path.
provider_fixture="$temp/provider-fixture"
"$root/tests/compatibility/bootstrap-harness-fixture.sh" --upgrade-cli --story US-INDEP-001 "$provider_fixture"
provider_cli="$provider_fixture/scripts/bin/harness-cli"; [[ -x "$provider_cli" ]] || provider_cli="$provider_fixture/scripts/bin/harness-cli.exe"
(cd "$provider_fixture" && "$provider_cli" tool register --name absent-proof --kind cli --capability optional-proof --command definitely-not-installed-us095 --scan definitely-not-installed-us095 --responsibility Verification --description "US-095 missing optional provider" --force >/dev/null)
(cd "$provider_fixture" && "$provider_cli" tool check --name absent-proof --json | jq -e 'length == 1 and .[0].name == "absent-proof" and .[0].status == "missing"' >/dev/null)
provider_doctor=$(cd "$temp" && "$bin" --repo-root "$provider_fixture" doctor)
rg -q '\[WARN\] optional providers.*absent-proof.*missing' <<<"$provider_doctor"
(cd "$temp" && "$bin" --repo-root "$provider_fixture" work list | rg -q US-INDEP-001)
(cd "$temp" && "$bin" --repo-root "$provider_fixture" run US-INDEP-001 --prepare-only >/dev/null)

run_fixture="$temp/run-fixture"
"$root/tests/compatibility/bootstrap-harness-fixture.sh" --upgrade-cli --story US-INDEP-001 "$run_fixture"
"$root/tests/compatibility/assert-contract-tuple.sh" "$run_fixture" "$tag"
completed=$(cd "$temp" && "$bin" --repo-root "$run_fixture" run US-INDEP-001)
run_id=$(sed -n 's/^Completed run //p' <<<"$completed")
[[ -n "$run_id" ]]
[[ -s "$run_fixture/.harness/runs/$run_id/SUMMARY.md" ]]
jq -e --arg run "$run_id" '.version == 1 and .run_id == $run and .story_id == "US-INDEP-001" and .outcome == "completed" and (.validation.commands | length == 1)' "$run_fixture/.harness/runs/$run_id/RESULT.json" >/dev/null
agent_events="$run_fixture/.symphony/worktrees/$run_id/.harness/runs/$run_id/AGENT_EVENTS.jsonl"
jq -e --arg run "$run_id" '.event == "fixture-agent.completed" and .run_id == $run and .story_id == "US-INDEP-001"' "$agent_events" >/dev/null
[[ -s "$run_fixture/.harness/runs/$run_id/RUN_CONTRACT.json" ]]
merge_sha=$("$root/tests/compatibility/review-and-merge-fixture-run.sh" "$run_fixture" "$run_id")
first=$("$root/tests/compatibility/assert-sync.sh" --expect applied "$bin" "$run_fixture" "$run_id")
second=$("$root/tests/compatibility/assert-sync.sh" --expect no-op --assert-state-unchanged "$bin" "$run_fixture" "$run_id")

# The Web scripts are maintained separately, but this story verifier composes
# them with the same external Harness fixture and release binary.
npm --prefix "$root/crates/harness-symphony/web-ui" run build
standalone="$temp/standalone"
"$root/tests/compatibility/build-minimal-standalone-bundle.sh" "$bin" "$standalone" >/dev/null
"$root/tests/compatibility/smoke-standalone-web.sh" "$standalone" "$run_fixture"
npm --prefix "$root/crates/harness-symphony/web-ui" run e2e
npm --prefix "$root/crates/harness-symphony/web-ui" run desktop:smoke -- --repo-root "$run_fixture"

[[ ! -e "$run_fixture/crates/harness-cli" && ! -e "$run_fixture/crates/harness-symphony" ]]
if rg -n '/repository-harness|/symphony|Documents/personal' "$run_fixture" --glob '!scripts/bin/harness-cli*' --glob '!.git/**'; then exit 1; fi
jq -n --arg tag "$tag" --arg prepare_hash "$before" --arg run_id "$run_id" --arg merge_sha "$merge_sha" --arg first "$first" --arg second "$second" '{story:"US-095",harness_release:$tag,prepare_root_logical_sha256:$prepare_hash,run_id:$run_id,local_merge_sha:$merge_sha,first_sync:$first,second_sync:$second}' >"$temp/evidence.json"
cat "$temp/evidence.json"
git -C "$root" diff --check
echo "US-095 cross-repository standalone parity verification passed"
