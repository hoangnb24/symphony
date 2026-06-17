# US-046 First-Class Symphony Codex Adapter

## Status

implemented

## Lane

normal

## Product Contract

Symphony treats Codex as a named agent adapter instead of requiring users to
wire `codex app-server` through the generic custom command adapter.

The `codex` adapter speaks the app-server JSON-RPC protocol over stdio:
initialize the connection, start a thread, start a turn with the prepared
worktree as the working directory, wait for `turn/completed`, then reuse the
existing Symphony result validation path.

The `custom` adapter remains available for arbitrary one-shot commands and for
future non-Codex experiments.

## Relevant Product Docs

- `docs/SYMPHONY_SCOPE.md`
- `docs/stories/epics/E05-symphony-local-runner/US-038-result-validation-agent-adapter.md`
- `docs/stories/epics/E07-symphony-automation/US-045-auto-mode-work-sources.md`

## Acceptance Criteria

- `agent.adapter: codex` is accepted by `doctor` and does not require an
  explicit `agent.command`.
- The Codex adapter defaults to `codex app-server`.
- The Codex adapter sends the documented app-server sequence:
  `initialize`, `initialized`, `thread/start`, `turn/start`.
- The Codex adapter waits for `turn/completed` before Symphony validates
  `SUMMARY.md`, `RESULT.json`, and changeset artifacts.
- `agent.adapter: custom` keeps the existing one-shot command behavior.
- Unsupported adapters fail with an actionable error.

## Design Notes

- Commands: `harness-symphony run <story-id>`
- Adapter boundary: `crates/harness-symphony/src/agent.rs`
- Existing validation remains in `crates/harness-symphony/src/run.rs`.
- Future adapters should be added beside `custom` and `codex` instead of
  special-casing command strings.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-046 --unit 1 --integration 1 --e2e 0 --platform 1`.

| Layer | Expected proof |
| --- | --- |
| Unit | Adapter command defaults, prompt rendering, and unsupported/missing command behavior. |
| Integration | Fake app-server JSON-RPC handshake test. |
| E2E | Live Codex app-server run can be exercised separately after artifact promotion is fixed. |
| Platform | `doctor` recognizes the Codex adapter on the local platform. |
| Release | Workspace tests, fmt, clippy. |

## Harness Delta

This story turns `agent.adapter` into the explicit extension point for Codex
and future agents. It also preserves the earlier finding that PR artifact
promotion is separate work, tracked by backlog item `#9`.

## Evidence

- `cargo test -p harness-symphony agent::tests -- --nocapture`
- `cargo test --workspace`
- `cargo fmt --check`
- `cargo clippy --workspace -- -D warnings`
- `cargo run -q -p harness-symphony -- doctor`
- `cargo run -q -p harness-symphony -- config show`

Live Codex app-server runs streamed progress and artifacts during research, but
this story does not claim E2E completion until the app-server emits the proper
`turn/completed` lifecycle event in the exercised run.
