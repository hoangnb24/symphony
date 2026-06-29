# US-047 Dependency-Aware Board State Foundation

## Status

implemented

## Lane

high-risk

## Product Contract

The Symphony Web UI Controller must show all Harness tasks as Ready, Blocked,
In Progress, Review, Needs Attention, or Done. Those states must be derived
from Harness stories, Harness-owned dependency edges, and existing Symphony run
state rather than from a separate Web UI task store.

## Relevant Product Docs

- `docs/product/symphony-web-ui-controller.md`

## Acceptance Criteria

- Harness has a typed direct story dependency edge table.
- Symphony can list direct blockers and tasks unblocked by a completed task.
- Symphony derives Ready and Blocked from incomplete blockers.
- Dependency cycles are detected and explained as task breakdown problems.
- Existing single-active-run state is reflected as In Progress.
- Completed, failed, PR review, and synced run states map to the Web UI board
  states.

## Design Notes

- Commands: `harness-symphony work board`.
- Queries: Harness `story`, Harness `story_dependency`, Symphony `run_state`.
- API: local web API deferred to `US-048`.
- Tables: `story_dependency`.
- Domain rules: dependencies live in Harness; Web UI does not create tasks.
- UI surfaces: backend model only in this story.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id <id> --unit 1 --integration 1 --e2e 0 --platform 0`.

| Layer | Expected proof |
| --- | --- |
| Unit | Board derivation tests. |
| Integration | CLI board command over a migrated DB. |
| E2E | Deferred to `US-053`. |
| Platform | Local CLI smoke. |
| Release | Not required. |

## Harness Delta

Adds the first Web UI Controller epic and a Harness dependency-edge schema
needed by feature intake and the Web UI board.

## Evidence

- `cargo test -p harness-symphony board` passed: 4 board derivation tests.
- `cargo test -p harness-symphony` passed: 50 tests.
- `cargo test --workspace` passed: 85 tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace -- -D warnings` passed.
- `target/debug/harness-symphony work board` rendered the repo board and showed
  `US-047` as `Ready` before the implementation status update.
- Harness DB migrated to schema version 7 and durable story `US-047` was marked
  implemented with unit, integration, and platform proof.
