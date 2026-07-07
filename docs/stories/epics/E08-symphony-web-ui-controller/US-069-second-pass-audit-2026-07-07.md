# US-069 Second-Pass Audit - Symphony Web UI Controller

## Metadata

- Date: 2026-07-07
- Scope: Symphony Web UI Controller second-pass production-readiness audit
- Mode: Review-only audit using `frontend-agents:frontend-production-workflow`
- Primary story: `US-069 Web UI Design Principles And Validation`
- Prior audit: `docs/stories/epics/E08-symphony-web-ui-controller/US-069-deep-audit-2026-07-07.md`
- Product contract: `docs/product/symphony-web-ui-controller.md`
- Frontend surface: `crates/harness-symphony/web-ui`
- Harness intake: `#171`

## Decision

Fix Required.

The second pass confirms the prior audit's blocking findings are still present
and adds new architecture, accessibility, and design findings. The controller
builds, its existing browser tests pass, and the desktop smoke passes, but it is
not production-ready under the frontend production workflow gate.

The strongest blocking risks are:

- Live board state does not refresh from active-run terminal state.
- Retry communication can show stale failed-run events.
- Review fetch failures are silent.
- `Open artifacts` is an enabled no-op.
- The custom dialog declares modal semantics but does not behave modally.
- Search, error, loading, and run-log states lack required accessibility
  semantics.
- Mobile first viewport hides the board below summaries.
- Long artifact/review values can force mobile dialog horizontal overflow.
- Feature state and API parsing are still concentrated in `main.tsx`.

## Validation Evidence

Passed:

```bash
npm --prefix crates/harness-symphony/web-ui run build
cargo test -p harness-symphony web -- --nocapture
npm --prefix crates/harness-symphony/web-ui run e2e
npm --prefix crates/harness-symphony/web-ui run desktop:smoke
```

Observed:

- Vite build passed with no Vite warnings.
- Main JS bundle remained `256.25 kB` raw / `78.77 kB` gzip.
- Rust web tests passed: 27 tests.
- Playwright E2E passed: 11 Chromium tests.
- Desktop smoke passed against an Electron-launched local backend.

Rendered fallback evidence:

- Browser plugin path was attempted first, but no in-app browser backend was
  available in this runtime (`agent.browsers.list()` returned `[]`), so the
  rendered pass used project Playwright.
- Built controller target: `http://127.0.0.1:43220`.
- Page identity: URL `/`, title `Symphony Controller`, H1 `Symphony work board`.
- Console/page errors: none observed in the rendered probes.
- Screenshots captured outside the repo:
  - `/tmp/symphony-audit-desktop.png`
  - `/tmp/symphony-audit-detail.png`
  - `/tmp/symphony-audit-mobile.png`
  - `/tmp/symphony-audit-long-mobile-detail.png`

Rendered checks:

- Six board columns rendered.
- Search textbox was not found by accessible name `Find task`.
- `Open artifacts` was enabled.
- Detail dialog had `role="dialog"` and `aria-modal="true"`.
- Focus left the dialog after four Tab presses and reached background content.
- At `390x760`, `#board` started at `y=889`, below the first viewport.
- A long artifact value made the mobile detail dialog `779px` wider than its
  client width.

## Tooling And Delegation

Configured reviewer fan-out was run through the frontend production workflow:

- `frontend-architecture-reviewer`: Fix Required.
- `react-quality-reviewer`: Fix Required.
- `shadcn-reviewer`: Fix Required.
- `accessibility-reviewer`: Fix Required.
- `vite-performance-reviewer`: Fix Required.
- `design-polish-reviewer`: Fix Required.

Skipped:

- `frontend-editor`: skipped because this was audit-only and no implementation
  edits were requested.
- In-app Browser runtime: attempted but unavailable; Playwright fallback used
  for rendered evidence.

Harness tool registry:

- No present provider for `accessibility`.
- No present provider for `performance-benchmark`.
- No present provider for `coverage`.

## UI Contract Reviewed

The audit checked the Web UI against the current product contract:

- Local-only controller over Harness/Symphony state, not a second source of
  truth.
- Dense operational board that keeps all task states visible.
- Start, observe, review, merge-mark, sync, recover, and inspect artifacts.
- Needs Attention must explain the failure and evidence path.
- Recovery must preserve failed-run evidence without confusing the active retry.
- Raw artifacts must remain accessible from review.
- Design validation must combine deterministic checks with visual review.

## Confirmed Findings

### Major - Board State Still Does Not Auto-Refresh During Active Runs

Evidence:

- `crates/harness-symphony/web-ui/src/main.tsx:167`
- `crates/harness-symphony/web-ui/src/main.tsx:184`
- `crates/harness-symphony/web-ui/src/main.tsx:214`
- `crates/harness-symphony/web-ui/src/main.tsx:413`

The board loads on mount and manual refresh only. `activeRun` is derived from
the current `items`, but no effect polls `/api/board` while a run is active and
no terminal event triggers a board refresh.

Impact:

- The board can keep showing stale `In Progress` state after the backend has
  moved a task to `Review` or `Needs Attention`.

Required fix:

- Poll or terminal-refresh `/api/board` while a run is active.
- Add Playwright coverage for `In Progress -> Review` and
  `In Progress -> Needs Attention` without manual refresh.

### Major - Active Retry Communication Can Prefer Stale Failed-Run Events

Evidence:

- `crates/harness-symphony/web-ui/src/main.tsx:923`
- `crates/harness-symphony/web-ui/src/main.tsx:925`
- `crates/harness-symphony/web-ui/src/main.tsx:1087`
- `crates/harness-symphony/web-ui/tests/board.spec.ts:409`

The component preserves prior failed review state during an execution retry,
then renders `review?.events ?? events`. Existing tests prove old failed
evidence remains visible, but not that the active retry's live events replace
stale failed-run communication.

Impact:

- The UI can show an active retry while the main communication area still reads
  like the old failed run.

Required fix:

- Separate preserved failed-run evidence from active-run live communication.
- Add E2E coverage proving new retry events render while old evidence remains
  available in a distinct panel.

### Major - Review Endpoint Failures Remain Silent

Evidence:

- `crates/harness-symphony/web-ui/src/main.tsx:931`
- `crates/harness-symphony/web-ui/src/main.tsx:934`
- `crates/harness-symphony/web-ui/src/main.tsx:1073`

The review effect only sets review state on `response.ok`; non-OK responses do
not create an explicit error state. The review panel simply does not render.

Impact:

- Users lose the exact evidence surface they need when review artifacts are
  incomplete or backend review fails.

Required fix:

- Add review loading and error state with HTTP status or malformed payload
  explanation.
- Add Playwright coverage for a failing review endpoint.

### Major - `Open artifacts` Remains An Enabled No-Op

Evidence:

- `crates/harness-symphony/web-ui/src/main.tsx:1048`
- `docs/product/symphony-web-ui-controller.md:184`
- Rendered probe: button was enabled in the detail dialog.

The button has no `onClick`, no link target, no disabled state, and no reason
explaining unavailable artifact access.

Impact:

- Dead control in a core review workflow.
- Violates the product contract that raw artifacts remain accessible.

Required fix:

- Wire artifact opening/access when artifact paths exist, or render the control
  disabled with a clear unavailable state.
- Add E2E coverage for the artifact control.

### Major - Dialog Declares Modal Semantics Without Modal Behavior

Evidence:

- `crates/harness-symphony/web-ui/src/main.tsx:188`
- `crates/harness-symphony/web-ui/src/main.tsx:882`
- `crates/harness-symphony/web-ui/src/main.tsx:988`
- Rendered probe: focus escaped the dialog after four Tab presses.

The detail popup sets `aria-modal="true"` and focuses the container, but it does
not trap focus, make background content inert, restore focus to the opener, or
scope Escape handling to the dialog.

Impact:

- Keyboard users can navigate behind a modal dialog.
- Screen-reader semantics overstate actual behavior.

Required fix:

- Use a real Dialog primitive or implement equivalent modal behavior.
- Add tests for Tab containment, Escape close, backdrop close, and focus
  restoration.

### Major - Accessibility State Semantics Are Still Missing

Evidence:

- `crates/harness-symphony/web-ui/src/main.tsx:156`
- `crates/harness-symphony/web-ui/src/main.tsx:404`
- `crates/harness-symphony/web-ui/src/main.tsx:422`
- `crates/harness-symphony/web-ui/src/main.tsx:429`
- `crates/harness-symphony/web-ui/src/main.tsx:1223`
- Rendered probe: `getByRole("textbox", { name: /find task/i })` found no
  control.

Search relies on placeholder text inside an empty label. Board loading does not
expose `aria-busy`; the error card has no `role="alert"`; run communication is
not a status/live region.

Impact:

- Assistive technology users may miss core search, loading, failure, and run
  progress state changes.

Required fix:

- Add a durable search label or `aria-label`.
- Add `aria-busy`, `role="alert"`, and appropriate live/status regions.
- Add accessibility-oriented Playwright assertions.

### Major - Mobile First Viewport Hides The Work Board

Evidence:

- `crates/harness-symphony/web-ui/src/main.tsx:390`
- `crates/harness-symphony/web-ui/src/main.tsx:420`
- Rendered probe at `390x760`: `#board` started at `y=889`.

On mobile, the source order renders sidebar, header, controls, and summary
cards before the board. The first viewport is mostly navigation and metrics,
not the primary task controller.

Impact:

- The main workflow is visually delayed on the smallest supported viewport.

Required fix:

- Rework mobile hierarchy so board or selected work is visible in the first
  viewport.
- Add a mobile layout assertion.

### Major - Long Detail And Review Values Can Overflow Mobile Dialogs

Evidence:

- `crates/harness-symphony/web-ui/src/main.tsx:1283`
- `crates/harness-symphony/web-ui/src/components/ui/badge.tsx:24`
- Rendered probe at `390x760`: dialog `scrollWidth - clientWidth = 779` for a
  long artifact path; one badge rendered about `1126px` wide.

`ListBlock` renders arbitrary artifact paths, changed files, blockers, and IDs
as fixed-height inline badges. Long values can widen the dialog instead of
wrapping inside it.

Impact:

- Review evidence can become hard to read on mobile and can cause horizontal
  scroll inside the modal.

Required fix:

- Render long detail/review values as wrapping rows or max-width chips.
- Add mobile overflow assertions for artifacts, changed files, blockers,
  children, and run IDs.

### Major - `main.tsx` Remains The Feature Boundary

Evidence:

- `crates/harness-symphony/web-ui/src/main.tsx` is 1,306 lines.
- It owns API DTOs, fetch actions, board layout, sidebar graph, detail dialog,
  review panel, event polling, run communication, and page composition.
- `docs/product/symphony-web-ui-controller.md:192`

The prior audit treated this as advisory. The second pass elevates it to major
because new problems cluster around missing boundaries: unsafe API casts,
unordered requests, dialog behavior, review state, and feature-specific tokens
all live in the same file.

Required fix:

- Extract feature-owned modules for API/parsing, board state, run events,
  board grid/card, sidebar graph, task detail dialog, review panel, and run
  communication.

### Major - API Payloads Are Cast Without A Parse Boundary

Evidence:

- `crates/harness-symphony/web-ui/src/main.tsx:55`
- `crates/harness-symphony/web-ui/src/main.tsx:175`
- `crates/harness-symphony/web-ui/src/main.tsx:899`
- `crates/harness-symphony/web-ui/src/main.tsx:935`
- `docs/ARCHITECTURE.md` parse-first boundary rule

The frontend casts unknown `/api/*` JSON directly to TypeScript response types.
There is no API client, parser, or normalizer boundary before data reaches
rendering code.

Impact:

- Malformed or partial responses become silent missing UI or unsafe assumptions
  instead of explicit error states.

Required fix:

- Add a frontend API boundary that validates and normalizes board, events, and
  review payloads.

### Major - Symphony State Semantics Leak Into Generic `Badge`

Evidence:

- `crates/harness-symphony/web-ui/src/components/ui/badge.tsx:4`
- `crates/harness-symphony/web-ui/src/components/ui/badge.tsx:6`
- `crates/harness-symphony/web-ui/src/main.tsx:704`
- `crates/harness-symphony/web-ui/src/main.tsx:1011`

`components/ui/Badge` includes feature tones such as `ready`, `blocked`,
`progress`, `review`, `attention`, and `done`. That makes the generic UI layer
Symphony-aware.

Impact:

- The design-system primitive is no longer product-agnostic.
- Status semantics and visual tokens are harder to reuse or audit cleanly.

Required fix:

- Keep `Badge` generic and token-based.
- Add a feature-level `StatusBadge` or status tone map for Symphony states.

### Major - Reduced Motion Does Not Cover Spinners

Evidence:

- `crates/harness-symphony/web-ui/src/main.tsx:414`
- `crates/harness-symphony/web-ui/src/main.tsx:701`
- `crates/harness-symphony/web-ui/src/main.tsx:1027`
- `crates/harness-symphony/web-ui/tests/board.spec.ts:92`

Reduced-motion coverage exists for close confetti, but loading and active-run
spinners still use `animate-spin`.

Impact:

- Motion-sensitive users still see continuous animation in core operational
  states.

Required fix:

- Suppress or replace nonessential spinner animation under reduced motion.
- Add reduced-motion tests for active and loading states, not only confetti.

## Advisory Findings

### Advisory - Fetch Concurrency Is Still Unordered And Not Abortable

Evidence:

- `crates/harness-symphony/web-ui/src/main.tsx:167`
- `crates/harness-symphony/web-ui/src/main.tsx:886`
- `crates/harness-symphony/web-ui/src/main.tsx:920`

Overlapping board/review/event requests can resolve out of order. Cancellation
flags avoid some unmount updates, but requests are not abortable or monotonic.

Recommended fix:

- Put ordering or `AbortController` guards in feature data hooks.
- Add stale-response tests.

### Advisory - Historical Event Polling Is Too Broad

Evidence:

- `crates/harness-symphony/web-ui/src/main.tsx:891`
- `crates/harness-symphony/web-ui/src/main.tsx:906`

The detail view polls events for `item.active_run ?? item.run_id`, including
historical review/done runs where the review payload already includes events.

Recommended fix:

- Poll only active runs.
- Use review events for historical runs.
- Stop or back off after terminal events.

### Advisory - Sidebar Links Are Misleading

Evidence:

- `crates/harness-symphony/web-ui/src/main.tsx:580`
- `crates/harness-symphony/web-ui/src/main.tsx:620`

Sidebar items render as anchors to `#board`, including `Run logs`, and do not
expose current section state.

Recommended fix:

- Use real navigation targets or buttons for non-navigation controls.
- Add `aria-current` or equivalent active section state.

### Advisory - Intermediate Widths Are Still Not Acceptance-Tested

Evidence:

- `crates/harness-symphony/web-ui/src/main.tsx:689`
- `crates/harness-symphony/web-ui/tests/board.spec.ts:285`
- `crates/harness-symphony/web-ui/tests/board.spec.ts:325`

The board intentionally uses a `min-w-[1120px]` grid outside mobile. Rendered
probes did not show page-level overflow at `900px`, but the intermediate
scrolling behavior still has no acceptance test.

Recommended fix:

- Add a tablet/split-screen viewport assertion and document whether horizontal
  board scrolling is intentional at that width.

### Advisory - Build-Only Vite Plugin Is In Runtime Dependencies

Evidence:

- `crates/harness-symphony/web-ui/package.json:19`
- `crates/harness-symphony/web-ui/vite.config.ts:2`

`@vitejs/plugin-react` is build tooling but is listed under runtime
dependencies.

Recommended fix:

- Move `@vitejs/plugin-react` to `devDependencies`.

## Existing Strengths

- Production build passes without Vite warnings.
- Rust web tests, Playwright E2E, and desktop smoke pass.
- Existing tests cover board columns, detail controls, delete behavior,
  dependency graph selection, dense card overflow, failure summaries, recovery,
  PR retry, and readable logs.
- Board card overflow handling remains solid at the tested desktop and mobile
  viewports.
- No relevant console or page errors were observed during the rendered probes.

## Recommended Fix Order

1. Fix live state correctness:
   - Active-run board refresh.
   - Active retry communication versus failed-run evidence.
   - Review fetch loading/error state.

2. Fix dead/misleading controls:
   - `Open artifacts`.
   - Sidebar link targets/current state.

3. Fix accessibility blockers:
   - Real dialog behavior.
   - Search accessible name.
   - Alert/status/live region semantics.
   - Reduced-motion spinner behavior.

4. Fix mobile and overflow polish:
   - Mobile first-viewport hierarchy.
   - Long detail/review value wrapping.
   - Intermediate viewport acceptance tests.

5. Refactor the feature boundary:
   - API/parsing/data hooks.
   - Feature-specific status badge.
   - Board, detail, review, and event-log modules.

6. Complete US-069 proof:
   - Screenshot or equivalent visual proof.
   - Design-tool clean skip when no provider is registered.
   - Durable story status update only after blockers are closed.

## Residual Risk

- No accessibility scanner, coverage provider, performance benchmark provider,
  or design-validation provider is registered in Harness.
- In-app Browser was unavailable, so rendered evidence came from project
  Playwright.
- The prior audit file is currently untracked in this checkout; this second
  pass is a separate document and does not modify that file.
- This was audit-only; no code fixes were attempted.
