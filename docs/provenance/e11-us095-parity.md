# US-095 Standalone Parity Evidence

The complete `scripts/verify-e11-us095.sh` gate passed on 2026-07-12 against
the public checksum-verified `harness-cli-v0.1.14` release after forcing an
independent fixture to upgrade from the checksum-verified v0.1.11 artifact.
The reproducible expectations are in
[`e11-us095-parity.json`](e11-us095-parity.json).

Run IDs, local fixture merge SHAs, and logical/content hashes are deliberately
ephemeral: every verifier run creates fresh repositories and prints those
values before deleting the fixture. They are not tracked as timeless claims.
The verifier instead checks the tracked expectation schema and proves the
operation counts, ordered operations, and hash relationships on every run.

## Cause And Effect Proof

- A third-directory release binary completed `doctor`, `work list`, and
  prepare-only against a fresh Harness repository without either product source
  tree on its runtime path.
- Prepare-only preserved the root logical database hash, Git HEAD and tracked
  status, and active changeset set. Only documented `.harness/runs/**` and
  `.symphony/**` local state appeared.
- A held SQLite reader kept a nonempty WAL while a trace committed. The Harness
  snapshot contained that exact trace, proving the snapshot did not omit an
  uncheckpointed commit.
- The fixture agent produced a matching run contract, deterministic event,
  summary, result, and exactly one four-line changeset: header followed by
  `story.update`, `story.verify`, and `story.complete` for `US-INDEP-001`.
- Local review merged the run. First sync applied exactly three operations;
  second sync applied zero, with identical logical and changeset-set hashes.
- Missing agents and legacy, malformed, or capability-incomplete Harness
  runtimes failed before root DB, branches, worktrees, changesets, or Symphony
  state changed.
- No optional provider produced a clean skip. A registered missing provider
  produced a warning while doctor, work listing, and prepare-only remained
  available.
- A binary-plus-`web-ui-dist` bundle served health, board JSON, the UI, and all
  referenced assets from outside both source trees, then terminated its entire
  process tree.
- Electron accepted the external fixture only after probing its complete pinned
  Harness contract and rejected both the retired crate-path decoy and a present
  but incompatible CLI.

## Retained Gates

- 119 Rust tests, formatting, and all-target Clippy with warnings denied.
- Protocol, architecture, WAL, and missing-agent mutation guards.
- Production Web build and all 19 Playwright tests.
- External-fixture desktop smoke.
- Linux/macOS/Windows CI now bootstraps a native external fixture and passes it
  explicitly to desktop smoke; Linux additionally runs the mutation fixtures.
