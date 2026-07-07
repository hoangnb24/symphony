# US-071 Reviewer Sub-Agent Closure

## Status

implemented

## Lane

normal

## Product Contract

Frontend and UI workflow changes must not be accepted with silent gaps in the
configured reviewer fan-out. When a workflow names reviewer sub-agents, every
configured reviewer must be run, explicitly skipped, or covered by a named
fallback before the work is marked complete.

## Relevant Evidence

- `docs/HARNESS.md` intentionally unchanged.
- `docs/HARNESS_COMPONENTS.md` intentionally unchanged.
- `docs/stories/epics/E08-symphony-web-ui-controller/US-069-deep-audit-2026-07-07.md`
- `docs/stories/epics/E08-symphony-web-ui-controller/US-069-second-pass-audit-2026-07-07.md`

## Acceptance Criteria

- `docs/HARNESS.md` and `docs/HARNESS_COMPONENTS.md` remain unchanged because
  they are reusable template files.
- Workflow-specific reviewer closure expectations remain in the story evidence
  that invokes them.
- A reviewer that was missed after implementation must be run again or recorded
  as an explicit skip before acceptance.
- `docs/HARNESS_COMPONENTS.md` remains unchanged and continues to describe the
  generic repository template state.
- The story evidence distinguishes an unavailable external design-validation
  provider from a missing `design-polish-reviewer` sub-agent pass.

## Design Notes

- Commands: no new CLI command in this slice.
- Queries: existing `query traces`, `query interventions`, and story/audit docs
  remain the evidence surfaces.
- API: not applicable.
- Tables: no schema change.
- Domain rules: reviewer closure is policy-level until a later story adds
  mechanical enforcement.
- UI surfaces: not applicable.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id <id> --unit 1 --integration 1 --e2e 0 --platform 0`.

| Layer | Expected proof |
| --- | --- |
| Unit | `rg` confirms the reviewer closure rule is documented in this story and not in template docs. |
| Integration | `scripts/bin/harness-cli query matrix` shows the durable story row and evidence. |
| E2E | Not applicable for policy-only docs. |
| Platform | Not applicable for policy-only docs. |
| Release | `git diff --check` passes. |

## Harness Delta

This story captures the reviewer-closure lesson for the Symphony Web UI story
line without changing the reusable Harness template files. The remaining gap is
mechanical enforcement: Harness can record traces, interventions, and stories,
but it does not yet inspect agent runtime delegation logs to prove each reviewer
actually ran.

## Evidence

- Intake: `#173`
- Validation passed:
  - `rg -n "reviewer closure|missing .*reviewer|explicit skip" docs/stories/epics/E08-symphony-web-ui-controller/US-071-reviewer-subagent-closure.md`
  - `git diff --quiet -- docs/HARNESS.md docs/HARNESS_COMPONENTS.md`
  - `scripts/bin/harness-cli story verify US-071`
  - `scripts/validate-changeset-rebuild.sh`
  - `git diff --check`
