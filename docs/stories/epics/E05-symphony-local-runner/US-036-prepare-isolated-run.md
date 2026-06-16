# US-036 Prepare Isolated Run Worktree

## Status

implemented

## Lane

normal

## Product Contract

`harness-symphony run <story-id> --prepare-only` must create an isolated
workspace for one eligible story and leave root `harness.db` unchanged.

## Relevant Product Docs

- `docs/SYMPHONY_SCOPE.md`
- `docs/ARCHITECTURE.md`

## Acceptance Criteria

- The root working tree is never used as the agent workspace.
- A dedicated worktree is created under `.symphony/worktrees/<run_id>/`.
- Root `harness.db` is copied into the worktree.
- The prepared environment includes `HARNESS_DB_PATH`, `HARNESS_RUN_ID`, and
  `HARNESS_RUN_MODE=execute`.
- The command refuses ineligible stories with clear reasons.
- Root `harness.db` checksum or row counts remain unchanged after prepare.

## Design Notes

- Command: `harness-symphony run <story-id> --prepare-only`.
- Uses git worktree operations.
- Depends on `US-028`.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-036 --unit 1 --integration 1 --e2e 1 --platform 1`.

| Layer | Expected proof |
| --- | --- |
| Unit | Run id/path generation and eligibility tests. |
| Integration | Temp git repo creates worktree and copied DB. |
| E2E | Prepare a real planned story and inspect artifacts. |
| Platform | macOS/Linux git worktree smoke. |
| Release | `cargo test --workspace`; `cargo fmt --check`; `cargo clippy --workspace -- -D warnings`. |

## Harness Delta

This is the MVP isolation behavior.

## Evidence

- Implemented `harness-symphony run <story-id> --prepare-only`.
- The command refuses stories whose status is not `planned` or `in_progress`.
- Prepare creates a git worktree under `.symphony/worktrees/<run_id>/`.
- Prepare copies the configured root Harness DB to `<worktree>/harness.db`.
- Prepare prints `HARNESS_DB_PATH`, `HARNESS_RUN_ID`, and
  `HARNESS_RUN_MODE=execute`.
- Prepare records local run state as `prepared`.
- `cargo test --workspace` passed with run contract and shim unit coverage.
- Temp git repo smoke created a worktree, copied DB, contract, AGENTS shim, and
  kept root story count unchanged at `1`.
