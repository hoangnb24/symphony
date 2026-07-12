# Design

## Domain Model

`HarnessProtocol` owns:

- resolved executable path;
- discovered contract/capabilities;
- read models for stories, dependencies, and hierarchy;
- typed mutation/apply results;
- transactionally consistent work-graph and DB-snapshot results;
- subprocess output/time/cancellation policy;
- compatibility errors.

Symphony services depend on this adapter, not a SQLite connection to
`harness.db`.

## Application Flow

```text
--repo-root
  -> resolve CLI candidates
  -> query contract JSON
  -> validate protocol/capabilities
  -> read one revisioned work-graph JSON
  -> snapshot root DB through SQLite-backup protocol when isolation is needed
  -> orchestrate Symphony-owned state
  -> issue Harness mutations through CLI
```

## Interface Contract

Compatibility errors include:

- executable not found;
- protocol too old/new;
- missing capability;
- schema requires migration;
- malformed JSON;
- CLI mutation failed.

They include the attempted path/version and one next action without dumping
sensitive environment values.

Each process call passes an executable and argument vector directly—never a
shell command string—sets the intended `cwd`, `HARNESS_REPO_ROOT`, and
`HARNESS_DB_PATH`, caps stdout/stderr, and applies the contract's read/mutation
timeouts. The resolved executable/argv is also represented structurally in the
run contract and generated agent prompt, so Windows and paths with spaces do
not fall back to `<repo>/scripts/bin/harness-cli` text.

## Data Model

Only `.symphony/state.db` remains directly managed by Symphony. Harness
`harness.db` is opaque behind the protocol. Run worktrees receive a consistent
Harness snapshot only from `db snapshot`; a file copy of the main DB, WAL, or
SHM is forbidden. Mutations still use the CLI with `HARNESS_DB_PATH`.

Before ownership transfer, copy the current target-owned E11 packets into the
target and commit `docs/provenance/e11-contract-packets.sha256`, mapping exact
source planning commit/path to exact target path/hash. Export an exact
source/target row/edge manifest for `US-093`-`US-096`: id, title, lane, status,
contract path, verify command, dependency edges, and runnable-owner state. Back
up both local databases and their matching replay epoch.

The cross-DB handoff uses a migration fence because no transaction spans both
files:

1. Stop/lock all work selectors, Web controllers, agents, and state writers for
   both DBs.
2. Before registering rows, write a crash-durable handoff-fence record in
   Symphony-owned `.symphony/state.db` and make command dispatch, automatic
   selection, direct run, Web controllers, and agents fail closed while it is
   held. Harness DB/schema is not extended for this temporary product fence.
3. Register target rows/edges with `status=planned` and `verify_command=NULL`.
   Prove target work selection returns zero and `run ... --prepare-only` is
   rejected even after restarting Symphony.
4. Land/test `changed -> Needs Attention` in the legacy source board and target
   board before changing any source proxy status.
5. Convert source copies to `status=changed` external-gate proxies and set
   their fail-closed verification to
   `scripts/verify-e11-external-gate.sh <story-id>`. Prove source automatic work
   selection excludes them, direct run rejects them, matrix retains visible
   coordination evidence, and the board labels them Needs Attention while all
   original source edges still exist. Existing
   `changed` scheduling semantics—not verifier failure—are the gate.
6. Configure/negative-test the target story verification wrapper, assert
   exactly one runnable owner, and only then release the fence.
7. On each target completion, write a receipt containing `version`, `story_id`,
   target repository, target commit SHA, protocol tag, validation run,
   `completed_at`, and release tag/manifest SHA when applicable. Verify its
   checksum plus owner signature/attestation in source, then explicitly use the
   existing verified `story complete` transition from `changed` for the
   matching proxy. First add a new detailed source trace whose changed-file and
   action fields name that receipt/target proof; never let the planning trace
   satisfy implementation completion.

Duplicate physical rows are permitted only as coordinated planning/proxy
evidence. While a target story is unfinished outside the fence, it has exactly
one runnable owner (target) and a non-runnable `changed` source proxy. After the target
receipt completes the source proxy, both records may be completed evidence and
neither is runnable. No selector ever observes two runnable owners.

The architecture test inventories and bans every prior Harness-internal path:

- `work.rs`: story/graph SQL reads and direct retire SQL;
- `run.rs`: story reads/status writes and raw DB file copy;
- `sync.rs`: `changeset_applied` SQL and human-output parsing;
- `doctor.rs`: fixed local CLI/copied-schema probes;
- `agent.rs`: fixed CLI path in the generated prompt.

The ban follows `ResolvedConfig.harness_db` into connections, SQL, and copies;
it does not ban `rusqlite` for the product-owned `.symphony/state.db`.

## UI / Platform Impact

Web/desktop surfaces display compatibility failures as setup problems, not task
failures. Windows executable discovery respects `.exe` and packaged paths.

## Observability

Doctor output records resolved CLI path, CLI version, protocol version, schema,
database compatibility state, and capability verdicts. It must not record
secrets or mutate the root DB.

## Alternatives Considered

1. Keep reads as SQL and route only writes through CLI. Rejected as the final
   boundary because schema changes would still silently break an independent
   release.
2. Copy `harness-cli` into the Symphony artifact. Rejected because Harness owns
   its release and target repositories already install it.
3. Link a shared Rust crate. Rejected because it couples product builds and
   release cadence.
