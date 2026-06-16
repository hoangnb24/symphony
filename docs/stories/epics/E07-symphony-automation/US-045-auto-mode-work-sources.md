# US-045 Auto-Mode Queue And Work Sources

## Status

implemented

## Lane

normal

## Product Contract

After local isolated runs are proven, Symphony may support unattended work
polling through a queue and work-source adapters.

## Relevant Product Docs

- `docs/SYMPHONY_SCOPE.md`

## Acceptance Criteria

- Auto-mode is explicitly opt-in.
- Queue and retry semantics are introduced only for unattended or concurrent
  work.
- `HarnessDbWorkSource` is the first work source.
- External sources such as GitHub Issues, Linear, Jira, and remote Harness are
  adapter boundaries, not changes to run contracts.
- Existing run contract, result, changeset, and sync semantics are reused.

## Design Notes

- This is v3 work and should not start before v1/v2 exit criteria pass.
- Potential adapters: `HarnessDbWorkSource`, `GitHubIssueWorkSource`,
  `LinearWorkSource`, `JiraWorkSource`, `RemoteHarnessWorkSource`.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-045 --unit 1 --integration 1 --e2e 1 --platform 1`.

| Layer | Expected proof |
| --- | --- |
| Unit | Queue eligibility, retry, and adapter contract tests. |
| Integration | Harness DB work source feeds one queued run. |
| E2E | Opt-in auto-mode processes a fixture story through the existing runner. |
| Platform | Long-running process smoke with graceful shutdown. |
| Release | `cargo test --workspace`; `cargo fmt --check`; `cargo clippy --workspace -- -D warnings`. |

## Harness Delta

This is the first step toward Symphony-style automation, deliberately sequenced
after the local workbench.

## Evidence

- Implemented `harness-symphony auto --enable` as the explicit opt-in entrypoint
  for unattended polling.
- Added auto config defaults for `source: harness-db`, `poll_interval_seconds`,
  and `max_attempts`.
- Added `HarnessDbWorkSource` as the first work-source adapter; GitHub Issues,
  Linear, Jira, and remote Harness are recognized as future adapter boundaries
  without changing run contracts.
- Added `.symphony/state.db` queue records with attempts, retry limit, terminal
  completion/failure state, and last error/run metadata.
- Auto mode reuses existing `execute_run`, isolated worktrees, `RUN_CONTRACT`,
  `RESULT.json`, changeset rendering, and run state updates.
- Unit proof: `cargo test -p harness-symphony` passed with 39 tests covering
  opt-in enforcement, adapter boundaries, queue attempts, retry, and
  `HarnessDbWorkSource` filtering.
- Integration/E2E proof: temp git repo smoke used `HarnessDbWorkSource` to feed
  `US-AUTO` into one queued isolated run with a fake agent; output reported
  `Enqueued: 1`, `Completed: 1`, `Failed: 0`, and `runs list` showed the run
  `completed`.
- Platform proof: idle long-running poll smoke exited cleanly via
  `--max-idle-cycles 1` with `Auto mode stopped: max idle cycles reached`.
- Release proof: `cargo test --workspace`, `cargo fmt --check`, and
  `cargo clippy --workspace -- -D warnings` passed.
