# US-038 Result Validation And Custom Command Adapter

## Status

implemented

## Lane

normal

## Product Contract

`harness-symphony run <story-id>` must be able to launch a configured local
agent command and accept the run only when required result artifacts are valid.

## Relevant Product Docs

- `docs/SYMPHONY_SCOPE.md`

## Acceptance Criteria

- Custom command adapter reads the configured command and executes it in the
  prepared workspace with the run environment.
- `RESULT.json` is required and must contain a valid v1 outcome.
- `SUMMARY.md` is required.
- Validation evidence is present or explicitly marked unavailable.
- Forbidden local runtime files are not staged for commit.
- Allowed outcomes are `completed`, `blocked`, `needs_intake`, `partial`,
  `failed`, and `cancelled`.

## Design Notes

- Command: `harness-symphony run <story-id>`.
- Adapter: custom command first; Codex can be one configured command, not a core
  dependency.
- Depends on `US-036` and `US-037`.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-038 --unit 1 --integration 1 --e2e 1 --platform 1`.

| Layer | Expected proof |
| --- | --- |
| Unit | RESULT schema validation and outcome tests. |
| Integration | Fake agent command writes valid/invalid artifacts and status updates. |
| E2E | A local run with a fake agent completes and is accepted. |
| Platform | Shell command adapter smoke. |
| Release | `cargo test --workspace`; `cargo fmt --check`; `cargo clippy --workspace -- -D warnings`. |

## Harness Delta

Defines the file-based finish protocol for local agent runs.

## Evidence

- Implemented `harness-symphony run <story-id>` for the `custom` command
  adapter.
- Agent command runs in the prepared worktree with `HARNESS_DB_PATH`,
  `HARNESS_RUN_ID`, and `HARNESS_RUN_MODE=execute`.
- `RESULT.json` and `SUMMARY.md` are required under
  `.harness/runs/<run_id>/` in the worktree.
- `RESULT.json` validates version, run id, story id, allowed v1 outcome,
  validation command evidence or explicit unavailable reason, and non-empty
  summary path when supplied.
- Forbidden staged paths (`harness.db`, `.symphony/state.db`,
  `.symphony/worktrees/**`) are rejected.
- Allowed outcomes are `completed`, `blocked`, `needs_intake`, `partial`,
  `failed`, and `cancelled`.
- `cargo test --workspace` passed: 35 `harness-cli` tests and 20
  `harness-symphony` tests.
- Fake-agent temp repo smoke completed a run with outcome `completed`, accepted
  valid result artifacts, and updated run state.
