# US-033 Symphony Doctor

## Status

implemented

## Lane

normal

## Product Contract

`harness-symphony doctor` must explain whether the repo is ready for isolated
Symphony runs and how to fix each failed prerequisite.

## Relevant Product Docs

- `docs/SYMPHONY_SCOPE.md`
- `docs/TOOL_REGISTRY.md`

## Acceptance Criteria

- Doctor checks git, worktree support, repo root discovery, database or rebuild
  readiness, `harness-cli` presence, `HARNESS_DB_PATH` support, operation-log
  support, `.gitignore` rules, agent adapter config, and PR adapter config when
  enabled.
- Each failure includes a concrete next command or config change.
- Missing optional PR support is a warning when PR creation is disabled.
- Doctor exits non-zero only for failed required checks.

## Design Notes

- Command: `harness-symphony doctor`.
- It should use `harness-cli` behavior checks rather than trusting docs.
- Capability lookup can be used for external adapters when registered.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-033 --unit 1 --integration 1 --e2e 1 --platform 1`.

| Layer | Expected proof |
| --- | --- |
| Unit | Individual check classification and remediation message tests. |
| Integration | Temp repo fixtures for pass, warning, and failure states. |
| E2E | Run doctor in this repo and capture output. |
| Platform | macOS/Linux shell smoke. |
| Release | `cargo test --workspace`; `cargo fmt --check`; `cargo clippy --workspace -- -D warnings`. |

## Harness Delta

Doctor becomes the first user-facing trust gate for Symphony.

## Evidence

- Implemented `harness-symphony doctor` with pass/warn/fail checks for git,
  git worktree support, repo root discovery, database/rebuild readiness,
  `harness-cli` presence, `HARNESS_DB_PATH` behavior, operation-log behavior,
  `.gitignore`, agent adapter configuration, and PR adapter availability.
- Doctor uses temp-repo behavior probes for `HARNESS_DB_PATH` and
  `HARNESS_RUN_ID` instead of trusting documentation.
- Added `.symphony/` to `.gitignore`.
- `cargo test --workspace` passed: 35 `harness-cli` tests and 8
  `harness-symphony` tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace -- -D warnings` passed.
- `git diff --check` passed.
- In-repo doctor smoke passed required checks and reported missing
  `agent.command` as a warning.
