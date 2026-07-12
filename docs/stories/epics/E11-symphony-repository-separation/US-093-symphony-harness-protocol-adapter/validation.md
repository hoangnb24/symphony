# Validation

## Proof Strategy

Use fake CLI processes for deterministic protocol edge cases and a real pinned
Harness release for behavioral parity.

## Test Plan

| Layer | Cases |
| --- | --- |
| Unit | CLI candidate ordering, version/capability comparison, JSON parsing, error redaction. |
| Integration | One-call revisioned work graph, WAL-safe snapshots, and CAS mutations through a fake/real CLI; replayed changeset parity. |
| E2E | Doctor, prepare, retire, execute, and sync against a disposable Harness project with no Symphony source. |
| Platform | Unix binary, Windows `.exe`, PATH lookup, configured path, and paths with spaces. |
| Performance | Work/board graph uses bounded CLI invocations rather than one process per row. |
| Logs/Audit | Incompatible preflight leaves canonical logical DB state and changeset content hashes unchanged. |
| Ownership | Staged target automatic selection/direct-run return zero/rejected; both source/target board tests map `changed` to Needs Attention; source proxy automatic selection/direct-run return zero/rejected while matrix retains visible evidence; then exactly one runnable target owner until a signed receipt permits verified source completion. |
| Architecture | No production connection/SQL/copy reaches `ResolvedConfig.harness_db`; product-owned `.symphony/state.db` remains allowed. |
| Recovery | Restart with the target fence held, inject failure after each handoff stage, and restore exactly one runnable owner before unlocking selectors. |

## Fixtures

- Compatible protocol v1 CLI.
- Missing, malformed, old, new/unsupported, and partial-capability CLIs.
- Disposable pinned Harness project.
- Applied/unapplied changesets.
- Paired pre-transfer source/target DB backups and exact row/edge manifest.
- Uncheckpointed WAL data, held-reader snapshot case, and source logical hashes.
- Checksummed target contract packet map and valid/invalid external-gate receipts.
- New receipt-backed detailed source trace; planning-trace-only completion must
  fail the gate.

## Commands

```bash
cargo fmt --check
cargo test -p harness-symphony --locked
cargo clippy -p harness-symphony --all-targets -- -D warnings
tests/compatibility/test-harness-protocol.sh
tests/compatibility/test-harness-wal-snapshot.sh
tests/architecture/no-direct-harness-db-access.sh
scripts/verify-e11-external-gate.sh US-093 <receipt>
git diff --check
```

## Acceptance Evidence

Pending implementation. Include logical before/after hashes for every negative
preflight case, one successful WAL snapshot and semantic replay, exact
source/target selection/ownership queries, contract packet hashes, signed
receipt verification, and a handoff rollback rehearsal.
