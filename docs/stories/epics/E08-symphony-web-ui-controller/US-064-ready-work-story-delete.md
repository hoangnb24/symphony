# US-064 Ready Work Story Delete Action

## Status

implemented

## Lane

normal

## Product Contract

The Symphony Web UI Controller should let users delete an unwanted Work Story
only when that story is in `Ready` state. The user-facing delete action retires
the Harness story so it leaves active work selection without physically deleting
durable Harness history.

## Relevant Product Docs

- `docs/product/symphony-web-ui-controller.md`

## Acceptance Criteria

- A delete button is available from the Work Story detail popup only when the
  selected task is currently `Ready`.
- Deleting requires an explicit confirmation that names the selected Work Story.
- Confirming delete re-checks server-side that the task is still `Ready` before
  changing durable state.
- Successful delete updates the Harness story status to `retired`, refreshes
  the board, and removes the task from active Ready work.
- Delete is refused for `Blocked`, `In Progress`, `Review`, `Needs Attention`,
  and `Done` tasks, even if a stale client attempts the request.
- Hard database deletion is out of scope; story rows, run artifacts,
  dependencies, hierarchy records, changesets, and validation history must not
  be physically removed.

## Design Notes

- Commands: `harness-symphony web`; durable mutation should reuse Harness story
  lifecycle semantics rather than bypassing the Harness database model.
- Queries: board derivation must treat `retired` stories as not active Ready
  work.
- API: add a scoped local endpoint such as
  `POST /api/tasks/<story-id>/retire` or equivalent; the route must reject
  non-Ready tasks.
- Tables: reuse `story.status = retired`; no schema change expected.
- Domain rules: Ready is a derived board state, so the backend must validate the
  derived current board state before retiring the story.
- UI surfaces: task detail popup in `crates/harness-symphony/web-ui`.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-064 --unit 1 --integration 1 --e2e 1 --platform 1`.

| Layer | Expected proof |
| --- | --- |
| Unit | Board/action tests cover Ready-only visibility, retired story filtering, and backend refusal for non-Ready states. |
| Integration | Web route tests prove successful Ready retirement and stale/non-Ready request rejection. |
| E2E | Playwright covers deleting a Ready Work Story from the task detail popup and verifies the board refresh removes it from active Ready work. |
| Platform | Browser build and Electron smoke/build prove the shared controller still packages after the action is added. |
| Release | Not required. |

## Harness Delta

No process change. The story clarifies that user-facing delete for Work Stories
means a reversible lifecycle retirement, not physical durable-state deletion.

## Evidence

- `cargo fmt --check`
- `cargo test -p harness-symphony retire -- --nocapture`
- `cargo test -p harness-symphony board_omits_retired_stories_from_active_work -- --nocapture`
- `cargo test -p harness-symphony web -- --nocapture`
- `npm --prefix crates/harness-symphony/web-ui run build`
- `npm --prefix crates/harness-symphony/web-ui run e2e`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- `npm --prefix crates/harness-symphony/web-ui run desktop:smoke`
- `git diff --check`
