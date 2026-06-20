# Validation

## Proof Strategy

Prove the board model independently before adding a browser. Tests should cover
dependency graph behavior and run-state overlays because those rules gate every
future Web UI action.

## Test Plan

| Layer | Cases |
| --- | --- |
| Unit | Ready item without blockers; blocked item with incomplete blocker; cycle detection; active run maps to In Progress; failed run maps to Needs Attention; completed PR run maps to Review; implemented story maps to Done. |
| Integration | `harness-symphony work board` reads a migrated Harness DB and Symphony state DB. |
| E2E | Deferred to Web UI stories. |
| Platform | Local CLI smoke on macOS/Linux path. |
| Performance | Not required for MVP story counts. |
| Logs/Audit | Harness trace records proof and friction. |

## Fixtures

- Temporary SQLite Harness database with `story` and `story_dependency`.
- Temporary Symphony state database with `run_state`.

## Commands

```text
cargo test -p harness-symphony board
cargo test --workspace
target/debug/harness-symphony work board
```

## Acceptance Evidence

- `cargo test -p harness-symphony board` passed: 4 board derivation tests.
- `cargo test -p harness-symphony` passed: 50 tests.
- `cargo test --workspace` passed: 85 tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace -- -D warnings` passed.
- `target/debug/harness-symphony work board` rendered the real repo board from
  Harness and Symphony state.
