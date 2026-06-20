# US-048 Local Web Backend Board API

## Status

implemented

## Lane

high-risk

## Product Contract

`harness-symphony web` must expose local-only APIs for the future browser UI.
The first API slice serves board data derived from Harness stories,
dependencies, and Symphony state. The backend must stay a controller over
existing Harness/Symphony truth, not a separate task runner or task store.

## Relevant Product Docs

- `docs/product/symphony-web-ui-controller.md`

## Acceptance Criteria

- `harness-symphony web` is a recognized command.
- The command binds to a local host by default.
- `GET /health` returns a simple OK response.
- `GET /api/board` returns JSON board items using the `US-047` board model.
- Unsupported paths return 404 JSON.
- The backend does not create tasks.

## Design Notes

- Commands: `harness-symphony web --host 127.0.0.1 --port 4317`.
- Queries: reuses `list_board`.
- API: `GET /health`, `GET /api/board`.
- Tables: no new tables beyond `story_dependency`.
- Domain rules: local-only, no auth for MVP.
- UI surfaces: no React UI in this story.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id <id> --unit 1 --integration 1 --e2e 0 --platform 0`.

| Layer | Expected proof |
| --- | --- |
| Unit | HTTP request routing and board JSON serialization tests. |
| Integration | CLI parser exposes `web`; local server can be built. |
| E2E | Deferred to browser stories. |
| Platform | Local command uses loopback host by default. |
| Release | Not required. |

## Harness Delta

None expected beyond durable story proof.

## Evidence

- `cargo test -p harness-symphony web` passed: 3 web routing/API tests.
- `cargo test -p harness-symphony` passed: 53 tests.
- `cargo test --workspace` passed: 88 tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace -- -D warnings` passed.
- Live loopback smoke started `harness-symphony web --host 127.0.0.1 --port
  43217`, verified `GET /health`, and verified `GET /api/board` returned
  board JSON containing `US-047` and `US-048`.
- Durable story `US-048` was marked implemented with unit, integration, and
  platform proof.
