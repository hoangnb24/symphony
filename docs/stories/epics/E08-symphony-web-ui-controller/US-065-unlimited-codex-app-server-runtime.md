# US-065 Unlimited Codex App Server Task Runtime

## Status

planned

## Lane

normal

## Product Contract

Symphony must not fail a task only because a Codex App Server run exceeded a
fixed wall-clock timeout. A Codex-backed task should keep running until Codex
reports a terminal turn, the app-server process exits, an explicit cancellation
path is added, a documented protocol stall guard fires, or required result
validation fails.

The existing `custom` adapter timeout behavior is out of scope unless the
implementation finds shared configuration that must be split to preserve the
Codex contract.

## Relevant Product Docs

- `docs/product/symphony-web-ui-controller.md`
- `docs/stories/US-046-first-class-symphony-codex-adapter.md`
- `docs/stories/epics/E08-symphony-web-ui-controller/US-050-run-start-event-api.md`

## Acceptance Criteria

- The Codex adapter no longer uses `agent_timeout_minutes` as a fixed
  wall-clock deadline for `codex app-server` turns.
- Long-running Codex tasks remain `In Progress` while the app-server is alive
  and no terminal turn, cancellation, protocol stall, or validation failure has
  occurred.
- Existing failure paths still produce actionable errors for app-server process
  exit, unsupported protocol requests, failed terminal turns, invalid result
  artifacts, and documented protocol stalls.
- Configuration, help, doctor, and docs no longer imply that Codex App Server
  tasks are capped at the old default timeout.
- The Web UI active-run event polling continues to expose events for a
  long-running Codex task without forcing the task into `Needs Attention`.

## Design Notes

- Commands: `harness-symphony run <story-id>`; `harness-symphony web`.
- Queries: `GET /api/runs/<run-id>/events`.
- API: no API shape change expected.
- Tables: reuses `run_state`.
- Domain rules: one active run remains enforced; unlimited runtime applies only
  to the active Codex-backed execution loop.
- UI surfaces: task detail active-run event stream and Needs Attention state.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-065 --unit 1 --integration 1 --e2e 1 --platform 1`.

| Layer | Expected proof |
| --- | --- |
| Unit | Codex adapter tests cover no fixed deadline and retained terminal failure handling. |
| Integration | Fake app-server run remains active past the former timeout boundary without failing solely on elapsed wall clock. |
| E2E | Web UI/start API proof shows an active Codex run remains in progress while events continue or the process is alive. |
| Platform | Local `doctor`/config output no longer presents Codex runtime as capped by `agent_timeout_minutes`. |
| Release | `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, and Web UI build/E2E if UI behavior changes. |

## Harness Delta

This story sharpens the Symphony Web UI controller contract: task runtime is
owned by the Codex App Server lifecycle and Symphony validation, not by an
arbitrary wall-clock cap.

## Evidence

Add commands, reports, screenshots, or links after validation exists.
