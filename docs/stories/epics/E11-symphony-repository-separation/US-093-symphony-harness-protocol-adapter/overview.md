# Overview

## Current Behavior

Symphony directly reads Harness tables, directly updates a Ready story to
retired, assumes `<repo>/scripts/bin/harness-cli`, copies schema files for a
probe, byte-copies a WAL-mode `harness.db`, and parses human changeset-apply
output. Its own `.symphony/state.db` is correctly product-owned, but Harness
state access is not isolated behind a versioned adapter.

## Target Behavior

Symphony has one Harness protocol adapter that discovers a compatible CLI,
preflights capabilities before mutation, consumes protocol v1 JSON, and routes
all Harness writes through the CLI. Internal Symphony state remains SQLite and
product-owned.

## Repository Ownership

Coordinated, with Symphony target code/DB as the primary implementation. The
source keeps non-runnable external-gate proxies until checksummed target
completion receipts satisfy the existing source dependency graph.

## Affected Users

- Operators running a standalone Symphony artifact against any Harness-enabled
  repository.
- Windows operators using `harness-cli.exe`.
- Maintainers diagnosing incompatible Harness versions.

## Affected Product Docs

- Target `docs/contracts/harness-runtime-v1.md`.
- Target Quickstart and troubleshooting docs.

## Acceptance Criteria

- CLI resolution order is explicit: configured path or `HARNESS_CLI_PATH`,
  target-local `scripts/bin/harness-cli`/`.exe`, then `PATH`.
- Before target initialization, Bash/PowerShell installation pins the Harness
  template source ref and CLI artifact to the exact immutable `US-092` release,
  uses the explicit checksum-verified CLI upgrade operation even if an older
  target-local binary exists, and asserts the complete contract tuple. No DB
  mutation occurs if source ref, artifact tag, checksum, or contract differs.
- Current target-owned E11 work packets (`US-093` through `US-096`) are copied
  after filtering and committed under target-owned paths before target stories
  reference them. A SHA-256 manifest records the source planning commit,
  destination path, and content hash; source copies remain until E11 closes.
- The target imports no legacy `.harness/changesets`. Any temporary semantic
  files emitted during the coordinated handoff are hashed as evidence and kept
  out of the target's active tracked tree; durable target rows plus committed
  contract/receipt evidence are the authority.
- A fresh target Harness DB is initialized through the released CLI before
  adapter work. Active target rows `US-093` through `US-096` plus the internal
  chain `US-093 -> US-094 -> US-095 -> US-096` are registered there; completed
  `US-090` and `US-091` are recorded as migration evidence rather than runnable
  work.
- During the locked handoff, target rows are mechanically staged as
  `status=planned AND verify_command IS NULL`; automatic selection returns zero
  and direct `run --prepare-only` is rejected by a fail-closed migration-fence
  record in Symphony-owned state. The guard is implemented/tested before target
  row registration and survives a process crash until explicitly released. Source
  `US-093` through `US-096` rows become `status=changed` non-runnable
  external-gate proxies with fail-closed receipt verification. Existing
  selector/direct-run semantics—not verifier failure—exclude `changed`; the
  Symphony board renders it as Needs Attention, never Ready. Before the fence
  releases, this board-state guard is landed and tested in both the target and
  still-present legacy source copy; the source cannot continue rendering a
  proxy as Ready. Matrix may retain the changed proxy as visible coordination
  evidence. The target is made runnable only after source automatic work
  selection returns zero.
- When target `US-093`, `US-094`, `US-095`, or `US-096` completes, an
  owner-signed/checksummed receipt is committed to the source and
  `scripts/verify-e11-external-gate.sh` verifies it before normal explicit
  `story complete` moves the matching source proxy from `changed` to completed.
  A new source trace linked to the proxy and replayable intake records the
  receipt path/hash, target commit, validation run, and reviewed action before
  completion; the original planning trace is not reused as implementation
  evidence.
  Source proxies are never retired, so source edges remain blocked until the
  real target proof arrives and completed E11 history remains queryable without
  becoming a suggestion.
- `doctor` checks protocol version and named capabilities before any persistent
  write and names the exact supported recovery/upgrade action.
- CLI `0.1.11` and schema `12` remain the legacy behavioral baseline. The
  positive support floor is the exact newer Harness CLI release produced by
  `US-092` with protocol-v1 capabilities; the accepted release tag and schema
  floor are recorded in the runtime contract. CLI `0.1.11` must be rejected as
  an upgrade-required negative fixture before mutation.
- Work, dependency, and hierarchy reads use the one-call, revisioned
  `work-graph.read.v1` protocol instead of direct Harness SQL.
- Ready-story retirement and copied-story status changes use Harness CLI
  writes and are replayable when a run ID is present.
- Sync consumes JSON apply results and never parses parentheses or words from
  terminal output.
- Doctor/status/sync inspect applied state through the read-only
  changeset-status protocol and never query `changeset_applied` directly.
- Every former `fs::copy` of a Harness DB in root/`--here`/worktree preparation
  is replaced with `isolated-db-snapshot.v1`; an uncheckpointed WAL commit is
  present in the snapshot and source logical state is unchanged.
- A coupling inventory closes every production path in `work.rs`, `run.rs`,
  `sync.rs`, `doctor.rs`, and `agent.rs`. An architecture test rejects direct
  SQL/connections/copies involving the resolved Harness DB while explicitly
  allowing SQLite access to `.symphony/state.db`.
- Every adapter invocation sets a deliberate current directory,
  `HARNESS_REPO_ROOT`, and `HARNESS_DB_PATH`. Run contracts and agent prompts
  carry the resolved executable plus argument vector rather than a shell-spaced
  command, including `.exe` and paths with spaces.
- Tests prove that a fake compatible CLI can drive Symphony and an incompatible
  CLI is rejected before the target's canonical logical-state hash changes.
  The adapter enforces the protocol's output ceiling and read/mutation timeout
  rules and returns stable compatibility/process errors.
- Windows `.exe` discovery and paths containing spaces pass.
- Symphony may continue using `rusqlite` for `.symphony/state.db`; architecture
  checks distinguish product-owned state from forbidden Harness DB access.
- All existing 99 Rust tests remain green with new adapter/compatibility tests.
- The only temporary source-product code mutation permitted by this story is
  the isolated `changed -> Needs Attention` board guard needed to keep legacy
  source UI safe during dual-copy operation; `US-098` later removes that copy.

## Non-Goals

- Remove Symphony's internal state database.
- Support an unbounded range of historical Harness versions.
- Embed or fork Harness CLI source.
- Change Web UI behavior beyond actionable compatibility errors.
