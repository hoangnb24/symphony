# Symphony Product Scope

Status: current standalone product contract

Symphony is an on-demand local orchestrator for executing Harness stories. It
turns typed work records into isolated agent runs, local review evidence, and
reviewable product/Harness changes. It is not the Harness policy engine, a
general-purpose issue tracker, or a hosted autonomous coding service.

## Product boundary

Harness owns story intent, lane, dependencies, hierarchy, verification policy,
and durable semantic operations. Symphony owns selection, run preparation,
agent launch, local run state, result validation, review surfaces, optional PR
automation, and post-merge changeset synchronization.

Cause and effect are explicit:

```text
Harness work graph
  -> Symphony selects a runnable story
  -> Harness creates a WAL-safe isolated database snapshot
  -> Symphony creates a worktree and RUN_CONTRACT.json
  -> the configured agent changes files and writes result evidence
  -> Harness records durable mutations as a semantic changeset
  -> Symphony validates and presents the run
  -> a human may accept its branch/PR
  -> Symphony asks Harness to apply the accepted changeset locally
```

Symphony is independently buildable and deployable. A target repository needs
a compatible Harness CLI artifact and database, but neither Harness source nor
a particular source-repository layout.

## Implemented and future matrix

| Area | Current contract | Future / out of current scope |
| --- | --- | --- |
| Invocation | Local CLI and Web/desktop controller; explicit `--repo-root`; checksum-verifiable local/CI release candidate | Remote publication (US-100), hosted service |
| Work discovery | One revisioned typed work-graph read with lane, status, dependencies, hierarchy, and revision | External issue trackers as the authoritative work model |
| Selection | Runnable work listing and explicit story runs; bounded unattended polling for opted-in work | Unbounded scheduler, distributed queue, multiple concurrent writers |
| Isolation | Git worktree for normal/high-risk; tiny `--here`; protocol-created WAL-safe DB snapshot | Container/VM sandboxing or remote execution |
| Agent runtime | Configured adapters, including Codex app-server behavior; explicit run contract and cancellation/status surfaces | Universal agent compatibility or automatic repair guarantees |
| Results | Required versioned `RESULT.json` and `SUMMARY.md`; validation before acceptance; local review/log artifacts | Treating result files as durable product state |
| Harness mutations | Protocol-routed compare-and-set writes and semantic operation logs | Direct SQL, table coupling, or deriving changesets by diffing databases |
| Review | Changed-file, validation, summary/result, event, and changeset views | Automatic approval without a human-controlled acceptance policy |
| Pull requests | Optional configured PR create/retry; summary supplies the body; branch carries product changes and semantic changeset | PR provider as a mandatory dependency |
| Sync | Idempotent protocol status/apply of committed changesets; local sync state | Committing or sharing `harness.db` / `.symphony/state.db` |
| Retention | Local run artifacts can be compacted; committed changesets remain durable | Using local logs as permanent cross-clone history |
| Configuration | Optional `.harness/symphony.yml`, version 1; tracked example | Requiring personal `.agents`, `.codex`, or `.impeccable` trees |
| Design tooling | May be used externally by contributors; absence does not block runtime | Bundled design-tool ownership or runtime dependency |

## Typed Harness protocol boundary

Symphony treats Harness as an external service exposed by an executable. The
supported tuple and capability list are pinned in the
[`Harness runtime contract`](contracts/harness-runtime-v1.md). In concrete
terms:

1. Symphony resolves one Harness CLI from configuration, environment, the
   target repository, or `PATH`.
2. Before a read or mutation, it requests a protocol-v1 discovery envelope and
   checks CLI version, schema range, protocol version, and named capabilities.
3. Work state comes from one `query work-graph --json` operation. Symphony
   does not assemble it from table reads.
4. Isolated databases come from `db snapshot --json`, so committed WAL pages
   are included consistently. Symphony does not byte-copy a live database.
5. Story writes use typed compare-and-set operations. Changeset inspection and
   replay use typed status/apply operations.
6. Every process gets explicit repository/database paths, timeouts, and an
   output-size bound. A malformed envelope, wrong operation, incompatible
   version, missing capability, timeout, overflow, or invalid exit/error pair
   fails closed before subsequent mutation.

Only `.symphony/state.db` is directly owned by Symphony. `harness.db` remains
opaque behind the Harness protocol.

## Run and artifact contract

Normal and high-risk runs use an isolated worktree. Tiny runs may use the
current checkout only when explicitly requested with `--here`; database
isolation and artifact validation still apply.

Each run receives a versioned `RUN_CONTRACT.json` containing its identity,
story, workspace/database paths, required outputs, allowed/forbidden paths, and
validation context. The agent must write a versioned `RESULT.json` with the
matching run/story identity and an allowed terminal outcome, plus a readable
`SUMMARY.md`.

Artifact durability is intentionally split:

| Artifact | Meaning | Durability |
| --- | --- | --- |
| Product/code/docs changes | Proposed product delta | Branch/PR |
| `.harness/changesets/*.changeset.jsonl` | Semantic Harness operations | Commit and retain |
| `SUMMARY.md`, `RESULT.json`, validation and event logs | Run evidence and review input | Local; compactable |
| `harness.db` | Harness local index | Local; rebuildable |
| `.symphony/state.db` | Symphony local controller state | Local only |

Therefore “successful run” does not mean “merged change.” A result can be
valid while its branch still awaits review. Likewise, PR acceptance does not
mutate another clone's database: after merge, `sync` detects the committed
changeset, asks Harness to apply it once, and records local sync state.

## Operational guarantees

- `doctor` reports actionable readiness failures, including an incompatible or
  missing Harness runtime.
- Normal/high-risk work cannot silently fall back to the root checkout.
- `--here` is rejected for non-tiny lanes.
- A protocol incompatibility fails before persistent Harness mutation.
- Result identity and schema are validated before a run is accepted.
- Sync is idempotent and does not mark failed application as successful.
- PR automation is optional; local execution and review remain usable without
  it.
- Personal tool configuration is not part of Symphony's product contract.

## Configuration and distribution

Configuration is target-repository-relative and optional. A normal Harness
repository with a compatible CLI and current database works with defaults;
desktop discovery validates that public contract rather than looking for
Symphony source. Start from
[`examples/symphony.yml`](../examples/symphony.yml), then create
`.harness/symphony.yml` in the target only for settings that differ from
defaults. CLI discovery and upgrade details are normative in the
[`runtime contract`](contracts/harness-runtime-v1.md).

The stable archive layout is `bin/harness-symphony(.exe)`,
`share/harness-symphony/web-ui/**`, and
`share/harness-symphony/resource-manifest.json`. The executable validates the
manifest paths and shape before serving packaged assets. The release verifier
recomputes the Web tree hash to validate the asset bytes. Local and CI release
candidates carry per-archive checksums and release provenance; remote
publication remains gated by US-100. Signing, notarization, and auto-update are
explicitly deferred.

## Explicit non-goals

- Reimplementing Harness intake, risk classification, or durable schema.
- Reading or writing Harness tables directly.
- Owning, vendoring, or copying Harness source.
- Treating local databases or run evidence as committed collaboration state.
- Requiring a PR provider, design tool, personal skill tree, or editor setup.
- Promising cross-machine scheduling, hosted execution, or packaged releases
  before their owning stories deliver them.
