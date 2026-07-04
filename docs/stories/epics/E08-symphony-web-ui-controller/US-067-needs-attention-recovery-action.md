# US-067 Needs Attention Recovery Action

## Status

implemented

## Lane

normal

## Product Contract

When a task is in `Needs Attention`, the Web UI must provide an explicit,
safe recovery path when the recorded failure is retryable. A user should be
able to inspect the failure summary, choose a guarded recovery action, and move
the story back into the normal lifecycle without losing the failed run's
artifacts or evidence.

Recovery must create or repair the next lifecycle step; it must not rewrite the
old failed run into a successful run.

## Relevant Product Docs

- `docs/product/symphony-web-ui-controller.md`
- `docs/stories/epics/E08-symphony-web-ui-controller/US-050-run-start-event-api.md`
- `docs/stories/epics/E08-symphony-web-ui-controller/US-051-review-surface-run-artifacts.md`
- `docs/stories/epics/E08-symphony-web-ui-controller/US-055-pr-merged-gate-before-sync.md`
- `docs/stories/epics/E08-symphony-web-ui-controller/US-066-needs-attention-failure-explanation.md`

## Acceptance Criteria

- A `Needs Attention` task shows a recovery action only when the backend marks
  the latest run state as retryable.
- Retryable agent, artifact, validation, cancelled, partial, or interrupted
  runs can be started again from the Web UI after an explicit confirmation.
- Retrying execution creates a new Symphony run for the same story, preserves
  the previous failed run and artifacts, and moves the board card to
  `In Progress` for the new active run.
- A completed run with failed or missing PR creation offers a PR retry action
  instead of starting the agent again.
- PR retry reuses the existing PR creation behavior and moves the task to
  `Review` when a PR is created; failures return to `Needs Attention` with the
  new failure explanation.
- Recovery is refused with a clear 409 or 400 response when another run is
  active, the story is no longer runnable, the latest state is not recoverable,
  or the task is already in `Review` or `Done`.
- The failed run remains accessible from the review/detail surface after a
  retry so the user can compare the old evidence with the new run.
- The UI labels recovery controls as retry/recover actions, not as a generic
  Start button, so the user understands this is a second attempt from a failed
  state.

## Design Notes

- Commands: `harness-symphony web`; no standalone CLI command is required for
  the MVP unless the implementation finds the web route should share a command
  handler.
- Queries: reuse board derivation over latest `run_state` records; keep old
  failed runs in `.symphony/state.db` and `.harness/runs`.
- API: add a guarded recovery route such as
  `POST /api/tasks/<story-id>/recover` for execution retry and a PR retry route
  such as `POST /api/runs/<run-id>/pr-retry`, or an equivalent shape that keeps
  execution retry distinct from PR retry.
- Tables: no schema change expected; `run_state` can keep multiple runs per
  story and board derivation already selects the latest run by creation time.
- Domain rules: recovery eligibility is derived on the backend from story
  status, latest run status, PR state, active-run lock, and sync state. The
  frontend must not guess recovery safety from display text.
- UI surfaces: Needs Attention board card, task detail popup, review evidence
  panel, and Electron shell through the shared React UI.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-067 --unit 1 --integration 1 --e2e 1 --platform 1`.

| Layer | Expected proof |
| --- | --- |
| Unit | Rust tests cover recovery eligibility for failed/cancelled/partial/validation failure, PR failure, active-run conflict, non-runnable story, Review, and Done states. |
| Integration | Web route tests prove execution retry creates a new run for a Needs Attention story, PR retry updates PR state, and refused recovery returns clear JSON errors. |
| E2E | Playwright verifies a mocked Needs Attention task shows retry controls, starts a recovery run, preserves the failure evidence, and uses PR retry for PR failures. |
| Platform | `npm --prefix crates/harness-symphony/web-ui run desktop:smoke` proves the shared Electron surface still loads recovery controls. |
| Release | `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, Web UI build, Web UI E2E, and `git diff --check`. |

## Harness Delta

This story closes the loop opened by `US-066`: `Needs Attention` becomes an
actionable recovery state, not only a failure explanation state.

## Evidence

- Backend recovery metadata and guarded routes implemented in
  `crates/harness-symphony/src/web.rs`:
  `POST /api/tasks/<story-id>/recover` for execution retry and
  `POST /api/runs/<run-id>/pr-retry` for PR retry.
- Shared React controller renders retry-specific actions from backend
  `recovery_action` fields and avoids generic Start controls for
  `Needs Attention` recovery.
- Validation passed:
  `cargo test -p harness-symphony web -- --nocapture`;
  `npm --prefix crates/harness-symphony/web-ui run build`;
  `npm --prefix crates/harness-symphony/web-ui run e2e`;
  `cargo fmt --check`;
  `git diff --check`;
  `cargo test --workspace`;
  `cargo clippy --workspace -- -D warnings`;
  `npm --prefix crates/harness-symphony/web-ui run desktop:smoke`.
