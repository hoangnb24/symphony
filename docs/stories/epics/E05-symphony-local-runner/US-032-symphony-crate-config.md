# US-032 Symphony Crate And Configuration

## Status

implemented

## Lane

normal

## Product Contract

The repository must expose a `harness-symphony` CLI entrypoint with typed
configuration loaded from `.harness/symphony.yml`.

## Relevant Product Docs

- `docs/SYMPHONY_SCOPE.md`
- `docs/ARCHITECTURE.md`

## Acceptance Criteria

- A `harness-symphony` binary can parse `--help` and known top-level commands.
- `.harness/symphony.yml` is optional; defaults match the scope.
- Config paths are normalized relative to the repo root.
- Invalid config produces actionable errors.
- No runner behavior is implemented before config and path boundaries are
  tested.

## Design Notes

- Suggested crate: `crates/harness-symphony`.
- Config path: `.harness/symphony.yml`.
- Avoid shared core crate until duplication becomes real.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-032 --unit 1 --integration 1 --e2e 0 --platform 1`.

| Layer | Expected proof |
| --- | --- |
| Unit | Config defaults, parsing, path normalization, and invalid config errors. |
| Integration | CLI help and config fixture smoke. |
| E2E | n/a. |
| Platform | Build binary from workspace on macOS/Linux. |
| Release | `cargo test --workspace`; `cargo fmt --check`; `cargo clippy --workspace -- -D warnings`. |

## Harness Delta

Introduces Symphony as a separate execution-isolation surface.

## Evidence

- Added `crates/harness-symphony` as a workspace binary crate with explicit
  `harness-symphony` binary target.
- Implemented typed config loading from optional `.harness/symphony.yml` with
  defaults matching `docs/SYMPHONY_SCOPE.md`.
- Implemented repo-root-relative path normalization and actionable parse/read
  errors.
- Added command stubs for `doctor`, `work list`, `run`, `runs list/show`,
  `status`, and `config show`; only config inspection is active in this story.
- `cargo test --workspace` passed: 35 `harness-cli` tests and 5
  `harness-symphony` tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace -- -D warnings` passed.
- CLI smoke verified `target/debug/harness-symphony --help`, default
  `config show`, fixture config normalization, and invalid config failure.
