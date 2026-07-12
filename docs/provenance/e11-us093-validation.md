# US-093 Validation Evidence

Validated on 2026-07-12 against Symphony implementation commit
`4a384543e43a2b9cb46f4baf5ed89bf1972a898f`, ownership record commit
`a383cd9`, and rollback-test commit `6c9805e`.

## Runtime contract and installation

- Forced upgrade source/tag: `harness-cli-v0.1.14`.
- Tag commit: `d2f89eeabe8d01df95fd19cd6ba981b01a71730f`.
- Local macOS-arm64 CLI SHA-256:
  `0adcd5360cd636c189fe0cd958e5b73261f7012a4e43631f08c61269c785caf9`.
- Before installation, CLI `0.1.11` rejected `query contract` with exit 2 and
  no target Harness database existed.
- After installation, read-only discovery returned CLI `0.1.14`, protocol 1,
  schema range 1–13, missing DB state, `HARNESS_DB_PATH`, and all ten required
  capabilities without creating the database.
- After explicit initialization, `doctor` reported schema 13 and the resolved
  executable path.

## Ownership handoff

- Exact source planning packets were committed first with
  `docs/provenance/e11-contract-packets.sha256`.
- Paired source/target backups are stored outside Git under
  `/Users/themrb/Documents/personal/e11-migration-artifacts/US-093-20260712`;
  the paired manifest SHA-256 is
  `84bfd6e94af7c4f7a41e88af583dce6bb6e0d33a793412bf895d33f3f2922066`.
- Target rows US-093–US-096 were staged as planned/null-verifier under the
  persistent fence, and target automatic/direct execution was rejected after
  process restart.
- Source rows became changed receipt proxies. Their complete dependency-set
  SHA remained
  `94a62565d9e3617db560603de51ea21fb8e46fb78b7a4c465b123526d54abdeb`.
- All four source board rows rendered Needs Attention; source direct execution
  rejected US-093 because `changed` is non-runnable.
- Every source receipt gate rejected before a receipt existed.
- `E11_US093_FORCE_FAILURE=1 scripts/verify-e11-us093.sh` failed as the
  deliberate target negative fixture.
- The normal verifier passed while the fence was held. Target US-093 then
  became the only generically runnable row; automatic and direct execution
  remained product-fenced.
- Both repositories were pushed before fence release. After release, source
  runnable count was zero and target runnable IDs were exactly `["US-093"]`.
- `tests/compatibility/test-ownership-handoff-rollback.sh` rehearsed target
  disable → zero-owner fenced interval → source restore → source disable →
  target restore entirely in WAL-safe snapshots. Canonical source and target
  logical hashes were unchanged.

## Adapter and regression proof

The following passed:

```text
cargo fmt --all --check
cargo test -p harness-symphony --locked                       110 passed
cargo clippy -p harness-symphony --all-targets -- -D warnings
tests/architecture/no-direct-harness-db-access.sh
tests/compatibility/test-harness-protocol.sh
tests/compatibility/test-harness-wal-snapshot.sh
tests/compatibility/test-ownership-handoff-rollback.sh
scripts/verify-e11-us093.sh
git diff --check
```

The compatibility suite proved CLI `0.1.11` rejection before state writes,
partial-capability rejection, paths with spaces, strict one-line JSON,
revisioned graph reads, runnable-checked CAS, content-SHA-bound sync,
post-pull rediscovery, and a WAL snapshot containing an uncheckpointed commit.
The architecture test found no production Harness SQL/connection/copy path in
`work.rs`, `run.rs`, `sync.rs`, `doctor.rs`, or `agent.rs`; direct SQLite remains
only in `state.rs` for `.symphony/state.db`.

An independent review found and then rechecked four integration risks. The
final implementation re-preflights after Git refresh, uses a per-changeset
`BEGIN IMMEDIATE` migration-fence guard, sets explicit `HARNESS_REPO_ROOT` for
agent processes, and binds Symphony sync state to content SHA. The final
review reported no remaining blocker.
