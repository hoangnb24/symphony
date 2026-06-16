# US-037 Run Contract And AGENTS Shim

## Status

implemented

## Lane

normal

## Product Contract

Every prepared run must include a machine-readable `RUN_CONTRACT.json` and a
visible worktree `AGENTS.md` shim that tells an agent exactly what to do.

## Relevant Product Docs

- `docs/SYMPHONY_SCOPE.md`
- `AGENTS.md`

## Acceptance Criteria

- `RUN_CONTRACT.json` contains version, run id, mode, story id, worktree,
  harness DB path, required outputs, forbidden paths, and agent instructions.
- The worktree `AGENTS.md` includes a short Symphony block linking to the
  contract and restating story id, DB path, required outputs, and forbidden
  paths.
- Existing project instructions remain readable.
- Contract paths are normalized and repo-relative where appropriate.

## Design Notes

- Output: `.symphony/runs/<run_id>/RUN_CONTRACT.json`.
- Worktree shim should be small and marked so it can be refreshed.
- Depends on `US-036`.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-037 --unit 1 --integration 1 --e2e 1 --platform 0`.

| Layer | Expected proof |
| --- | --- |
| Unit | Contract serialization and shim rendering tests. |
| Integration | Prepare-run fixture writes expected JSON and AGENTS block. |
| E2E | Agent-readable contract exists in a prepared worktree. |
| Platform | n/a. |
| Release | `cargo test --workspace`; `cargo fmt --check`; `cargo clippy --workspace -- -D warnings`. |

## Harness Delta

Makes the run contract visible through the file agents reliably read first.

## Evidence

- Implemented `RUN_CONTRACT.json` generation under
  `.symphony/runs/<run_id>/RUN_CONTRACT.json`.
- Contract contains version, run id, mode, story id, worktree, harness DB path,
  required outputs, forbidden paths, agent instructions, and explicit Harness
  environment values.
- Implemented worktree `AGENTS.md` Symphony block with contract path, story id,
  harness DB path, required outputs, forbidden paths, and environment values.
- Existing `AGENTS.md` content is preserved and the Symphony block is appended.
- `cargo test --workspace` passed with contract serialization and AGENTS shim
  assertions.
- Temp git repo smoke verified contract file exists and worktree `AGENTS.md`
  contains `HARNESS-SYMPHONY:BEGIN`.
