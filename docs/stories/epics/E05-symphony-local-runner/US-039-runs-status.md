# US-039 Runs List Show And Status

## Status

implemented

## Lane

normal

## Product Contract

Symphony must show local run state and the next human action without requiring
users to inspect `.symphony/state.db`.

## Relevant Product Docs

- `docs/SYMPHONY_SCOPE.md`

## Acceptance Criteria

- `harness-symphony runs list` shows run id, story id, branch, worktree, status,
  result path, PR URL when present, sync status, and next action.
- `harness-symphony runs show <run_id>` shows detailed metadata and artifact
  paths.
- `harness-symphony status` warns about active runs and unapplied committed
  changesets when detection exists.
- Missing local runtime state is reported as no active runs, not as a crash.

## Design Notes

- Commands: `runs list`, `runs show`, `status`.
- Uses the state store from `US-035`.
- Unapplied changeset checks can start as a stub until sync exists.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-039 --unit 1 --integration 1 --e2e 0 --platform 1`.

| Layer | Expected proof |
| --- | --- |
| Unit | Status rendering and next-action tests. |
| Integration | State DB fixture produces expected list/show output. |
| E2E | n/a until full run is stable. |
| Platform | CLI smoke. |
| Release | `cargo test --workspace`; `cargo fmt --check`; `cargo clippy --workspace -- -D warnings`. |

## Harness Delta

Gives humans a clear local control panel for v1 runs.

## Evidence

- Implemented `harness-symphony runs list` from `.symphony/state.db`.
- Implemented `harness-symphony runs show <run_id>` with detailed run metadata
  and artifact paths.
- Implemented `harness-symphony status`; missing runtime state initializes
  cleanly and reports no active run instead of crashing.
- `runs list` shows run id, story id, branch, worktree, status, result path,
  PR URL, sync status, and next action.
- Unapplied changeset detection remains a future `sync` concern; v1 state
  records carry `sync_status`.
- `cargo test --workspace` passed: 35 `harness-cli` tests and 20
  `harness-symphony` tests.
- CLI smokes verified no-active status, empty runs table, completed fake-agent
  run listing, and `runs show` for a prepared temp run.
