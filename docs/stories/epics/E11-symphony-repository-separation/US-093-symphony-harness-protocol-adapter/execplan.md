# Exec Plan

## Goal

Replace implicit filesystem/SQLite/prose coupling with one tested public
protocol adapter in the standalone target.

## Scope

In scope:

- Target Harness DB initialization and replay-safe registration of target-owned
  rows/edges.
- Post-filter transfer/checksum of target-owned E11 contract packets.
- Non-runnable source external-gate proxies and receipt-based completion.
- CLI resolution and compatibility preflight.
- One-call graph reads, WAL-safe DB snapshots, and CAS JSON writes.
- Removal of direct Harness table mutation/read paths.
- Cross-platform and negative-version tests.

Out of scope:

- Symphony state-store replacement.
- UI redesign.
- Broader Harness version support than the tested contract.

## Risk Classification

Risk flags:

- Public contract.
- Existing behavior.
- Data model.
- Cross-platform.
- Weak proof around mutation ordering.

Hard gates:

- Durable state mutation.
- Compatibility failure before write.

## Work Phases

1. Commit checksummed target copies of the current E11 work packets.
2. Force-upgrade/install the exact `US-092` Harness tag, verify contract tuple,
   then initialize target Harness state.
3. Hold the cross-DB fence, implement/restart-test the target fail-closed
   product-state fence, stage target rows, land/test the temporary changed-proxy
   board guard in both copies, convert source rows to `changed` receipt proxies,
   configure the first target verifier, and prove one runnable owner.
4. Add adapter types and bounded fake protocol process fixtures.
5. Implement executable/argv discovery and non-mutating preflight.
6. Move work/graph reads and all root/worktree snapshots behind the adapter.
7. Move retirement/status/apply mutations behind CAS/JSON adapter operations.
8. Remove prose parsing and every direct Harness DB access path; add the
   architecture test.
9. Run source, WAL, receipt-fence, rollback, and negative compatibility tests.

## Dependencies

- `US-091` standalone workspace.
- `US-092` published Harness protocol.

## Stop Conditions

Pause if any root mutation still requires SQL, protocol errors occur after a
write, any DB isolation still uses file copy, the adapter requires Harness
source, Windows resolution cannot be tested, either DB backup cannot be
verified, target contract hashes differ, a row/edge differs from the ownership
manifest, the migration fence cannot stop every selector/direct run/writer, or
any unfinished story has zero/two runnable owners after the fence is released.

## Rollback

On any failure after phase 3, reacquire the cross-DB fence and stop both writers
before repair or paired-backup restoration; never assume the earlier fence is
still held. If the handoff is abandoned, make target authority non-runnable,
restore the source proxy to `planned`, configure/verify its wrapper, and prove
source is the only runnable owner before unlocking. If target activation
already occurred, fence first and make target non-runnable before restoring
source. Never retire the source proxy as a shortcut. Every rollback ends with
explicit source/target automatic-selection and direct-run assertions before
the fence releases. Source cleanup remains blocked until adapter parity and
the required target receipts pass.
