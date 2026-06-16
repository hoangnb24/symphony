# US-035 Run State Store And Active-Run Lock

## Status

implemented

## Lane

normal

## Product Contract

Symphony v1 must allow only one active local run and record run metadata in
local ignored state.

## Relevant Product Docs

- `docs/SYMPHONY_SCOPE.md`

## Acceptance Criteria

- `.symphony/state.db` records run id, story id, branch, worktree path, status,
  result path, PR URL when present, sync status, and next action.
- Starting or preparing a second active run fails with an actionable message.
- Completed, cancelled, or failed runs release the active-run lock according to
  documented state transitions.
- `.symphony/` is ignored by git.

## Design Notes

- Runtime state is local and disposable.
- No queue is introduced in v1.
- Run statuses are separate from Harness story statuses.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-035 --unit 1 --integration 1 --e2e 0 --platform 1`.

| Layer | Expected proof |
| --- | --- |
| Unit | State transitions and lock conflict tests. |
| Integration | SQLite state store create/read/update smoke. |
| E2E | n/a until run command exists. |
| Platform | Git ignore check for `.symphony/`. |
| Release | `cargo test --workspace`; `cargo fmt --check`; `cargo clippy --workspace -- -D warnings`. |

## Harness Delta

Keeps v1 deterministic and avoids premature queue behavior.

## Evidence

- Implemented local `RunStateStore` backed by `.symphony/state.db`.
- State records include run id, story id, branch, worktree, status, result
  path, PR URL, sync status, and next action.
- Added single active-run lock for `prepared` and `running` statuses.
- `completed`, `failed`, and `cancelled` statuses release the active-run lock.
- Wired `runs list`, `runs show`, and `status` to read local state.
- Added `.symphony/` to `.gitignore`; `git check-ignore` confirms state DB and
  worktree paths are ignored.
- `cargo test --workspace` passed: 35 `harness-cli` tests and 16
  `harness-symphony` tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace -- -D warnings` passed.
- CLI smoke verified `status` reports no active run and `runs list` renders an
  empty local state table.
