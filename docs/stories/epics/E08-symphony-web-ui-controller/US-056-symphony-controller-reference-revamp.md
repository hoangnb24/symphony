# US-056 Symphony Controller Reference Revamp

## Status

implemented

## Lane

normal

## Product Contract

The Symphony Web UI should adopt the reference controller design in
`docs/design/symphony-web-ui-controller/` while preserving the existing local
API contracts for board, run event, review, PR merged, and sync state.

## Relevant Product Docs

- `docs/product/symphony-web-ui-controller.md`
- `docs/design/symphony-web-ui-controller/README.md`

## Acceptance Criteria

- The browser surface uses a Notion-style Symphony sidebar, summary strip,
  six-state Kanban board, selected-task detail rail, and dependency/log review
  sections matching the reference template.
- The UI continues to read board data from `GET /api/board`.
- Ready tasks can still call `POST /api/tasks/<story-id>/start`.
- Run logs, review evidence, PR merged, and approve sync controls still use the
  existing run APIs.
- ShadCN-style primitives are used for application controls and framed UI.
- Browser E2E still proves the board, filtering, detail panel, hierarchy,
  dependency labels, and Start control.

## Design Notes

- Commands: `harness-symphony web`, Vite build and Playwright E2E.
- Queries: `GET /api/board`, `GET /api/runs/<run-id>/events`,
  `GET /api/runs/<run-id>/review`.
- API: no new runtime API.
- Tables: no new tables.
- Domain rules: visual revamp only; task state derivation remains backend-owned.
- UI surfaces: local React controller in `crates/harness-symphony/web-ui`.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-056 --unit 1 --integration 1 --e2e 1 --platform 1`.

| Layer | Expected proof |
| --- | --- |
| Unit | TypeScript build. |
| Integration | Vite production build. |
| E2E | Playwright board/detail flow. |
| Platform | Browser screenshot inspection against the supplied reference image. |
| Release | Not required. |

## Harness Delta

No harness process change expected.

## Evidence

- `npm --prefix crates/harness-symphony/web-ui run build` passed.
- `npm --prefix crates/harness-symphony/web-ui run e2e` passed.
- `git diff --check` passed.
- `scripts/bin/harness-cli story verify US-056` passed.
- Playwright viewport screenshots were captured for desktop, selected detail,
  and mobile controller states.
- Visual inspection compared the selected-detail and mobile screenshots against
  `docs/design/symphony-web-ui-controller/mqum833g-drawing-2026-06-26T07-34-24-936Z.png`.
