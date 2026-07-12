# US-094 Symphony Product Docs And Optional Tooling Migration

## Status

planned

## Owner Repository

`symphony`

## Lane

normal with source-of-truth review.

## Product Contract

The target contains Symphony's current product contract, operator/contributor
guides, architecture/runtime contract, configuration example, and selected
historical story evidence. It contains no project-local `.agents`, `.codex`, or
`.impeccable` dependency.

## Dependencies

- `US-093` completed in the target, including target Harness ownership and the
  accepted protocol adapter.

## Relevant Product Docs

- Source `docs/SYMPHONY_SCOPE.md`
- Source `docs/SYMPHONY_QUICKSTART.md`
- Source `docs/product/symphony-web-ui-controller.md`
- Source E05-E08 stories and US-046

## Acceptance Criteria

- Target README explains what Symphony is, how it relates to Harness, and how
  to install/run an artifact against `--repo-root`.
- Quickstart separates operator commands from contributor `target/debug`
  commands; operator steps do not assume repository-harness source.
- Scope status reflects implemented versus future behavior and no longer says
  the product belongs in repository-harness.
- `docs/contracts/harness-runtime-v1.md` names the pinned protocol and upgrade
  behavior.
- A tracked example configuration exists; user-local `.harness/symphony.yml`
  and `.impeccable/config.local.json` are not copied as personal state.
- E05-E08 and US-046 history is imported as historical/completed evidence, not
  blindly reactivated as planned work.
- Live-only Symphony backlog items (`#10`, `#11`, `#12`, `#14`) receive a
  reviewed target disposition and provenance note, but this normal docs story
  performs no durable mutation. High-risk `US-097` applies each disposition
  once after export/identity checks.
- Impeccable/design tooling is documented as an optional external provider.
  Absence is a clean skip.
- The intake-griller content is archived or packaged outside project-local
  `.codex`; it is not an execution prerequisite.
- `git ls-files` in the target returns no `.agents/**`, `.codex/**`, or
  `.impeccable/**` path.
- All internal documentation links and command examples pass automated checks.
- No source document is deleted from repository-harness in this story.

## Design Notes

- Preserve historical provenance from `US-090`, then reorganize in a normal
  target commit.
- Use an explicit historical section for retired `US-061` and `US-063`.
- Link to released Harness documentation rather than copying Harness CLI
  implementation docs.

## Validation

| Layer | Expected proof |
| --- | --- |
| Unit | Markdown link/command-path checks. |
| Integration | Example config parses and resolves against a fixture repo. |
| E2E | A new operator follows install -> doctor -> work list from outside source. |
| Platform | Bash and PowerShell examples use correct binary names. |
| Release | Packaged docs refer only to shipped files or stable external URLs. |

```bash
tests/docs/assert-symphony-product-boundary.sh
<target-doc-link-check>
cargo test -p harness-symphony config --locked
git diff --check
```

The boundary script distinguishes `rg` no-match from tool/I/O failure and fails
closed on obsolete source-path instructions or any tracked hidden tool tree.

## Harness Delta

Use the target Harness instance initialized by `US-093`. Do not reinstall over
it or copy source `harness.db` or its mixed operation log.

## Evidence

Pending implementation.
