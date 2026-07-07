# US-070 Readable Done Column Task Cards

## Status

planned

## Lane

normal

## Product Contract

The Symphony Web UI Done column must keep completed work-item cards readable
even when the column contains many implemented stories. Cards should stay
bounded inside the column and scroll vertically, but they must not collapse into
thin clipped strips that hide the title and metadata.

## Relevant Product Docs

- `docs/product/symphony-web-ui-controller.md`
- `docs/stories/epics/E08-symphony-web-ui-controller/US-058-scrollable-board-columns.md`
- `docs/stories/epics/E08-symphony-web-ui-controller/US-068-bounded-work-item-cards.md`
- `docs/stories/epics/E08-symphony-web-ui-controller/US-069-web-ui-design-principles-validation.md`

## Acceptance Criteria

- Done-column task cards use a readable compact-card height instead of
  collapsing to roughly one text line when many Done items are present.
- Each visible Done card exposes, at minimum, the story ID, verify/status badge,
  readable title summary, and key metadata such as lane and run/no-run state.
- Dense Done lists continue to scroll vertically inside the Done column without
  expanding the whole board or creating horizontal overflow.
- The card treatment remains visually consistent with the existing Symphony Web
  UI design language and the generated preview direction: compact, bounded,
  full-width cards with clear top, title, and metadata rows.
- Full work-item content remains in the task detail popup; the board card
  remains a summary.
- Existing board APIs, state derivation, task actions, review actions, recovery
  actions, and sync behavior are unchanged.

## Design Notes

- Commands: `harness-symphony web`, Vite build, Playwright E2E, Electron smoke.
- Queries: `GET /api/board`.
- API: no new API shape.
- Tables: no new tables.
- Domain rules: visual layout only; board state derivation remains
  backend-owned.
- UI surfaces: `crates/harness-symphony/web-ui/src/features/symphony/board.tsx`
  task list layout and task card sizing; existing card text bounding in
  `crates/harness-symphony/web-ui/src/styles.css`.
- Observed trigger: live Web UI at `http://127.0.0.1:4317` showed Done cards
  around `26px` tall while card content needed about `147-187px`, causing the
  title and metadata to be clipped.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-070 --unit 1 --integration 1 --e2e 1 --platform 1`.

| Layer | Expected proof |
| --- | --- |
| Unit | TypeScript build compiles the card/list layout changes. |
| Integration | Vite production build succeeds with the existing Web UI bundle. |
| E2E | Playwright covers a dense Done column and asserts visible Done cards have readable minimum height, no horizontal page/column/card overflow, and expected summary text/pills are visible. |
| Platform | Screenshot evidence for desktop and mobile shows dense Done cards remain readable and vertically scroll inside the column. |
| Release | `npm --prefix crates/harness-symphony/web-ui run build`, `npm --prefix crates/harness-symphony/web-ui run e2e`, `npm --prefix crates/harness-symphony/web-ui run desktop:smoke`, and `git diff --check`. |

## Harness Delta

No Harness process change expected. This story sharpens the existing bounded-card
validation pattern so future overflow checks cover both horizontal bounds and
vertical readability.

## Evidence

Planning evidence:

- Live QA screenshot showed the Done column with 63 tasks and collapsed cards.
- Generated image preview established the target visual direction: readable,
  compact Done cards with story ID, verify badge, title summary, and metadata
  rows.
