# US-034 Symphony Work List

## Status

implemented

## Lane

normal

## Product Contract

`harness-symphony work list` must show which Harness stories can be run and why.

## Relevant Product Docs

- `docs/SYMPHONY_SCOPE.md`
- `docs/HARNESS.md`

## Acceptance Criteria

- Work list shows story id, status, lane, verification state, runnable posture,
  and reason.
- Status values align with the current story schema:
  `planned`, `in_progress`, `implemented`, `changed`, `retired`.
- Missing verification command is a warning, not an invented story status.
- Implemented or retired stories are not shown as runnable by default.

## Design Notes

- Command: `harness-symphony work list`.
- Source: `harness.db` through `harness-cli` or a stable internal query.
- Domain rule: Symphony must not become a second intake classifier.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-034 --unit 1 --integration 1 --e2e 0 --platform 1`.

| Layer | Expected proof |
| --- | --- |
| Unit | Runnable classification tests for status/lane/verify combinations. |
| Integration | Temp DB fixture renders expected table rows. |
| E2E | n/a until run command exists. |
| Platform | CLI smoke in repo with current stories. |
| Release | `cargo test --workspace`; `cargo fmt --check`; `cargo clippy --workspace -- -D warnings`. |

## Harness Delta

Creates the first Symphony planning surface for humans.

## Evidence

- Implemented `harness-symphony work list` backed by read-only SQLite queries
  against the configured Harness DB.
- Output includes story id, status, lane, verification state, runnable posture,
  and reason.
- Missing verification commands render as `warn` with reason
  `proof command missing`; story status is left unchanged.
- `implemented` and `retired` stories render as not runnable.
- `cargo test --workspace` passed: 35 `harness-cli` tests and 12
  `harness-symphony` tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace -- -D warnings` passed.
- `git diff --check` passed.
- In-repo `target/debug/harness-symphony work list` smoke rendered the expected
  columns and current story rows.
