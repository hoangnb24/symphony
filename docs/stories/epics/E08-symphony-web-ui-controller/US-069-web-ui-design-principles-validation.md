# US-069 Web UI Design Principles And Validation

## Status

implemented

## Lane

normal

## Product Contract

The Symphony Web UI should have a lightweight component and design-principles
contract that guides future controller work without becoming a heavyweight
design-system project. Local shadcn-style primitives remain the component
foundation; Impeccable or equivalent design tooling may be used as a review and
anti-drift layer.

## Relevant Product Docs

- `docs/product/symphony-web-ui-controller.md`

## Acceptance Criteria

- A lightweight Web UI design contract exists for the Symphony controller.
- The contract defines the controller as a dense product/tool UI, not a
  marketing surface.
- The contract states when to use local shadcn-style primitives and when to
  extract product-specific components.
- The contract captures board/card/detail principles: bounded summaries,
  full detail in popups or panels, no nested card-heavy page sections, stable
  status tones, accessible focus states, and responsive overflow constraints.
- The contract documents how Impeccable can help: design vocabulary, audit,
  polish, anti-pattern detection, and optional CLI/browser review.
- The validation path includes existing build and Playwright checks, plus a
  clean skip or documented gap when design-validation tooling is not registered
  or installed.

## Design Notes

- Commands: `harness-symphony web`, Vite build, Playwright E2E, optional
  `npx impeccable detect crates/harness-symphony/web-ui/src/` if available.
- Queries: no runtime data query changes.
- API: no new API shape.
- Tables: no new tables.
- Domain rules: design guidance only; Harness and Symphony remain the state
  owners.
- UI surfaces: Web UI component primitives, task board, task detail popup,
  review/log surfaces, and future Electron shell builds.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-069 --unit 1 --integration 1 --e2e 1 --platform 1`.

| Layer | Expected proof |
| --- | --- |
| Unit | Documentation and component references are internally consistent. |
| Integration | Web UI build still succeeds after any component extraction or token changes. |
| E2E | Existing Playwright board/detail coverage still passes, with new checks only if the story changes rendered UI. |
| Platform | Screenshot review or equivalent visual proof demonstrates that the principles fit the controller on desktop and mobile. |
| Release | Not required. |

## Harness Delta

This story may add a reusable design-validation capability recommendation to
the Harness tool registry docs or backlog if Impeccable adoption requires a
new repeatable workflow.

## Evidence

Implemented after the 2026-07-07 deep and second-pass frontend audits.

Closed audit issues:

- Active-run board polling refreshes terminal `Review` and `Needs Attention`
  state without manual refresh.
- Active retry communication renders live retry events while prior failed-run
  evidence remains in a distinct panel.
- Review endpoint failures render explicit loading/error states with alert
  semantics.
- `Open artifacts` is no longer an enabled no-op; it is disabled with a clear
  unavailable reason while artifact paths remain visible in review evidence.
- Task detail dialog traps focus, closes on Escape/backdrop, and restores focus
  to the opener.
- Search, board loading, errors, and live run updates expose durable accessible
  names/status semantics.
- Reduced-motion mode suppresses operational spinner animation.
- Mobile layout brings the board into the first viewport, and long review/detail
  values remain bounded.
- Symphony status tones moved out of the generic `Badge` primitive into
  feature-owned status mapping.
- API responses now pass through a typed frontend parse boundary before state
  updates.
- Board/sidebar/detail/review/event-log/status/API modules were extracted from
  the `main.tsx` composition surface.

Validation passed:

```bash
npm --prefix crates/harness-symphony/web-ui run build
npm --prefix crates/harness-symphony/web-ui run e2e
cargo test -p harness-symphony web -- --nocapture
cargo test --workspace
cargo fmt --check
cargo clippy --workspace -- -D warnings
npm --prefix crates/harness-symphony/web-ui run desktop:smoke
git diff --check
```

Design-validation tooling was a clean skip: Harness has no present registered
provider for `accessibility`, `performance-benchmark`, `coverage`, or
design-validation, so deterministic Playwright desktop/mobile behavior and
overflow assertions are the current platform proof.
