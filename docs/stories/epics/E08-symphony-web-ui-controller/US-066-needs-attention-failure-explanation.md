# US-066 Needs Attention Failure Explanation

## Status

planned

## Lane

normal

## Product Contract

When a task moves to `Needs Attention`, the Web UI must make the failure
actionable from the board/detail surface. The user should see a concise reason,
the latest useful error or event, links to available run artifacts, and a
suggested next action without having to manually inspect `.harness/runs`.

This story does not change the underlying Codex timeout/runtime policy from
`US-065`; it explains whatever terminal failure Symphony recorded.

## Relevant Product Docs

- `docs/product/symphony-web-ui-controller.md`
- `docs/stories/epics/E08-symphony-web-ui-controller/US-050-run-start-event-api.md`
- `docs/stories/epics/E08-symphony-web-ui-controller/US-051-review-surface-run-artifacts.md`
- `docs/stories/epics/E08-symphony-web-ui-controller/US-060-human-readable-chat-logs.md`
- `docs/stories/epics/E08-symphony-web-ui-controller/US-065-unlimited-codex-app-server-runtime.md`

## Acceptance Criteria

- Board cards or the task detail header show a concise Needs Attention reason,
  not only the generic status.
- The detail/review panel shows the failure category, latest useful error or
  Codex event, and the run id.
- The detail/review panel links or names available evidence artifacts including
  `APP_SERVER_EVENTS.jsonl`, `SUMMARY.md`, `RESULT.json`, validation output,
  changeset output, or PR creation output when present.
- The UI recommends a next action such as inspect logs, retry when safe, fix
  validation, create/retry PR, wait for `US-065`, or handle manually.
- Missing or malformed artifact files degrade to a clear explanation instead
  of hiding the reason.
- A run that fails with a Codex app-server timeout similar to the US-064
  example surfaces the timeout message and `APP_SERVER_EVENTS.jsonl` path in
  the Web UI.

## Design Notes

- Commands: `harness-symphony web`.
- Queries: reuse or extend `GET /api/board`, `GET /api/runs/<run-id>/events`,
  and `GET /api/runs/<run-id>/review`.
- API: add presentation-safe failure summary fields to the board or review
  payload if the existing fields cannot support the UI without client-side
  guessing.
- Tables: reuse `run_state`; no schema change expected unless the current run
  state does not persist enough failure context.
- Domain rules: failure summaries are derived from Symphony run state and run
  artifacts; they must not become a second source of truth.
- UI surfaces: board card status detail, task detail popup, review/Needs
  Attention panel, and Electron shell through the shared React UI.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-066 --unit 1 --integration 1 --e2e 1 --platform 1`.

| Layer | Expected proof |
| --- | --- |
| Unit | Rust or React formatter tests cover timeout, missing artifact, PR failure, validation failure, and malformed event cases. |
| Integration | Web route fixture returns a Needs Attention run with concise reason, artifact paths, latest useful error/event, and suggested next action. |
| E2E | Playwright verifies the Web UI displays the reason and artifact evidence for a mocked Needs Attention run. |
| Platform | `npm --prefix crates/harness-symphony/web-ui run desktop:smoke` proves the shared Electron surface still loads. |
| Release | `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, Web UI build, Web UI E2E, and `git diff --check`. |

## Harness Delta

This story sharpens the controller failure-attribution contract: `Needs
Attention` is not just a lifecycle bucket; it is a user-facing diagnosis entry
point backed by run artifacts.

## Evidence

Add commands, reports, screenshots, or links after validation exists.
