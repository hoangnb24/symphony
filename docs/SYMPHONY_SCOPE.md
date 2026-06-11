# Harness Symphony Revised Scope

Status: proposed replacement scope

Audience: `repository-harness` maintainers, Harness CLI implementers, and
coding-agent runtime integrators.

Supersedes: the broader Symphony proposal from PR #16.

## 1. Product Thesis

Harness Symphony should not start as a full agent-orchestration platform.

It should start as a safe local agent workbench that turns Harness stories into
isolated, reviewable agent runs.

The useful promise is:

```text
Harness story
  -> isolated worktree
  -> copied harness.db
  -> explicit agent run contract
  -> validation/result artifact
  -> optional PR
  -> explicit post-merge reconciliation
```

This is narrower than OpenAI Symphony. OpenAI Symphony makes an issue tracker
the control plane for continuous autonomous implementation. Harness Symphony can
grow toward that model, but the first version should prove the Harness-specific
loop before adding daemon scheduling, external issue trackers, concurrent
agents, or automatic repair.

## 2. Why This Belongs In repository-harness

`repository-harness` already defines the agent operating system:

- intake and risk lanes
- context loading rules
- story and proof records
- trace and friction records
- verification commands
- drift/audit/proposal loops

The missing layer is execution isolation. Today an agent or human can follow the
Harness workflow, but the repository does not provide a repeatable way to:

- prepare a safe workspace for one story
- prevent accidental root `harness.db` mutation
- hand an agent a precise run contract
- capture run outcome in machine-readable form
- preserve useful artifacts for review
- reconcile durable state only after human acceptance

Harness Symphony should fill that gap.

## 3. Design Principles

### 3.1 Symphony Is Not A Second Harness Brain

Symphony must not deeply classify product work, decide implementation strategy,
assemble all task context, or replace `harness-cli`.

Harness owns:

- product intake
- risk lane policy
- story scope
- context rules
- validation expectations
- traces
- durable project memory

Symphony owns:

- run preparation
- workspace isolation
- copied database wiring
- agent launch contract
- run status and logs
- run result collection
- optional PR creation
- post-merge reconciliation support

### 3.2 The First Version Optimizes Trust, Not Throughput

The first useful version is not a daemon. It is a deterministic local runner
that humans and agents can understand.

Throughput features such as auto-polling, external work sources, multiple
active agents, and CI repair should wait until the local run contract is proven.

### 3.3 Committed Artifacts Are The Team Surface

`harness.db` is local operational state. It is not the collaboration surface.

The reviewable collaboration surface should be committed artifacts:

- run summaries
- run results
- semantic Harness changesets
- docs, stories, decisions, tests, and product changes

Long term, `harness.db` should be rebuildable from committed Harness artifacts
and changesets. Treating the local database as the only source of truth makes
team use brittle.

## 4. Revised v1 Scope

v1 is a safe on-demand runner.

### Required v1 Capabilities

#### 4.1 Doctor

Command:

```bash
harness-symphony doctor
```

Checks:

- Git is available.
- Git worktrees are supported.
- repository root is discoverable.
- `harness.db` exists or can be initialized.
- `harness-cli` exists or has a documented install/build path.
- `.gitignore` protects local DB and Symphony runtime files.
- configured agent adapter exists.
- configured PR adapter is available only if PR creation is enabled.

Doctor output should be actionable. Each failure should include the next command
or configuration change required to fix it.

#### 4.2 Work List

Command:

```bash
harness-symphony work list
```

Shows Harness work that can be run:

```text
ID      Status       Lane       Verify      Runnable  Reason
US-015  planned      normal     configured  yes       ready
US-016  in_progress  normal     missing     warn      proof command missing
```

v1 should align with the current `story.status` schema:

```text
planned
in_progress
implemented
changed
retired
```

If Symphony needs blocked, needs-intake, in-review, or done semantics, it must
add an explicit schema migration or store those states in a separate run/result
record. It must not silently assume statuses that the Harness database does not
support.

#### 4.3 Prepare Run

Command:

```bash
harness-symphony run <story-id> --prepare-only
```

Creates:

```text
.symphony/worktrees/<run_id>/
.symphony/runs/<run_id>/RUN_CONTRACT.json
.symphony/runs/<run_id>/harness.base.db
```

The root working tree is never used as the agent workspace.

The copied database is the only database the run should mutate. Symphony should
set:

```bash
HARNESS_DB_PATH=<worktree>/harness.db
HARNESS_RUN_ID=<run_id>
HARNESS_RUN_MODE=execute
```

`harness-cli` should learn to respect `HARNESS_DB_PATH` before Symphony depends
on copied database isolation.

#### 4.4 Run Contract

Each run must have a machine-readable contract:

```json
{
  "version": 1,
  "run_id": "run_123",
  "mode": "execute",
  "story_id": "US-015",
  "worktree": ".symphony/worktrees/run_123",
  "harness_db_path": ".symphony/worktrees/run_123/harness.db",
  "required_outputs": [
    ".harness/runs/run_123/SUMMARY.md",
    ".harness/runs/run_123/RESULT.json"
  ],
  "forbidden_paths": [
    "harness.db",
    ".symphony/state.db",
    ".symphony/worktrees/**"
  ],
  "agent_instructions": [
    "Follow AGENTS.md and Harness docs.",
    "Implement only the assigned story scope.",
    "Use the copied harness.db.",
    "Run the configured verification command when available."
  ]
}
```

This contract is for agents first and humans second. It should remove ambiguity
about where the agent is allowed to work and what it must produce.

#### 4.5 Agent Launch

Command:

```bash
harness-symphony run <story-id>
```

v1 may support one default local agent adapter plus a custom command adapter.

Configuration example:

```yaml
agent:
  adapter: custom
  command:
    - "codex"
    - "app-server"
```

Codex app-server can be the first tested adapter, but it should not be a domain
requirement. Harness is meant to support multiple coding agents.

#### 4.6 Finish Protocol

Agents should not signal success only by exiting.

Required output:

```text
.harness/runs/<run_id>/RESULT.json
```

Example:

```json
{
  "version": 1,
  "run_id": "run_123",
  "story_id": "US-015",
  "outcome": "completed",
  "validation": {
    "commands": [
      {
        "command": "cargo test --workspace",
        "result": "pass"
      }
    ]
  },
  "changed_files": [
    "crates/harness-cli/src/interface.rs"
  ],
  "summary_path": ".harness/runs/run_123/SUMMARY.md",
  "harness_db_changed": true
}
```

Allowed v1 outcomes:

```text
completed
blocked
needs_intake
partial
failed
cancelled
```

`blocked` and `needs_intake` are run outcomes in v1. They should not be written
into `story.status` unless the Harness schema explicitly supports them.

#### 4.7 Local Status

Command:

```bash
harness-symphony runs list
harness-symphony runs show <run_id>
```

Shows:

- run id
- story id
- branch
- worktree
- status
- result path
- PR URL if created
- reconciliation status
- next human action

The CLI should make unreconciled merged PRs obvious.

#### 4.8 Optional PR Creation

PR creation should be configurable, not mandatory for every run.

```yaml
pull_request:
  create: ask
  draft_for:
    - blocked
    - needs_intake
    - partial
```

Recommended v1 policy:

- completed implementation: open normal PR
- intake-only: open draft PR
- blocked/needs-intake: open draft PR only if useful artifacts exist
- failed/cancelled: no PR by default

If a PR is created, it must include:

```text
.harness/runs/<run_id>/SUMMARY.md
.harness/runs/<run_id>/RESULT.json
```

It may include:

```text
.harness/changesets/<run_id>.changeset.jsonl
```

only after semantic changeset generation is implemented.

#### 4.9 Semantic Changesets

Raw SQLite diffs are not a good v1 review surface.

Changesets should be semantic Harness operations:

```jsonl
{"op":"changeset.header","version":1,"run_id":"run_123","base_schema_version":4}
{"op":"story.update","id":"US-015","payload":{"status":"in_progress"}}
{"op":"trace.add","payload":{"story_id":"US-015","outcome":"completed"}}
```

Each operation should be:

- stable
- idempotent where possible
- schema-versioned
- reviewable in PR
- applyable through `harness-cli`, not direct SQL

v1 may defer changeset application until after the run contract and result
protocol work.

#### 4.10 Manual Reconciliation

Command:

```bash
harness-symphony reconcile <pr-number>
```

Preconditions:

- PR exists.
- PR is merged.
- merge commit is present locally.
- changeset exists if the run produced one.
- run has not already been reconciled.
- root `harness.db` schema is compatible.

Reconciliation must not partially corrupt root `harness.db`.

Longer term, add:

```bash
harness-cli db rebuild --from .harness/changesets
```

so a new clone can reconstruct durable Harness state from committed artifacts.

## 5. v1 Non-Goals

Do not include these in v1:

- automatic work polling
- multiple active runs
- Linear, GitHub Issues, Jira, or external work-source adapters
- hosted dashboard
- webhook reconciliation
- CI repair mode
- review-comment repair mode
- automatic PR merge
- multi-agent planning
- raw SQLite merge through Git

These are future features after the local workbench is useful.

## 6. v2 Scope: Reviewable PR Runner

Add after v1 proves that agents can complete local isolated runs.

Required:

- PR creation adapter
- draft/open PR policy
- semantic changeset generation
- manual reconciliation
- unreconciled PR detection
- branch cleanup command

Commands:

```bash
harness-symphony pr create <run_id>
harness-symphony pr retry <run_id>
harness-symphony reconcile <pr-number>
harness-symphony status
```

Acceptance:

- a completed run can become a PR
- a merged PR can update root `harness.db` through a semantic changeset
- a closed-unmerged PR leaves root `harness.db` untouched
- `status` shows unreconciled merged PRs

## 7. v3 Scope: Local Queue

Add only after PR/reconciliation works.

Required:

- run request queue
- one active run lock
- retry command
- stale run detection
- `prepare -> run -> finish -> optional PR` lifecycle through the queue

Commands:

```bash
harness-symphony queue list
harness-symphony queue cancel <request-id>
harness-symphony runs retry <run-id>
```

Still no auto-mode by default.

## 8. v4 Scope: Symphony-Style Automation

This is where the project starts to resemble OpenAI Symphony.

Required:

- auto-mode
- polling Harness work source
- policy-driven eligibility
- external work-source adapter interface
- bounded concurrency if run isolation is proven

Potential adapters:

```text
HarnessDbWorkSource
GitHubIssueWorkSource
LinearWorkSource
JiraWorkSource
RemoteHarnessWorkSource
```

The adapter boundary should not change run contracts, result files, workspace
isolation, or reconciliation semantics.

## 9. Architecture Boundaries

Suggested crates:

```text
crates/
  harness-core/
    domain/
    ports/
    use_cases/

  harness-cli/
    existing durable-layer CLI
    semantic changeset commands
    db rebuild commands

  harness-symphony/
    cli/
    config/
    adapters/
    orchestration/
```

Dependency direction:

```text
harness-symphony -> harness-core
harness-cli      -> harness-core
harness-core     -> no infrastructure dependencies
```

Do not split crates before the first implementation needs shared domain code.
If the MVP can be implemented cleanly in one crate first, prefer the smaller
change.

## 10. Configuration

Path:

```text
.harness/symphony.yml
```

Example:

```yaml
version: 1

repo:
  root: "."
  harness_db: "harness.db"

symphony:
  state_db: ".symphony/state.db"
  runs_dir: ".symphony/runs"
  worktrees_dir: ".symphony/worktrees"
  single_active_run: true

agent:
  adapter: custom
  command:
    - "codex"
    - "app-server"
  timeout_minutes: 120

pull_request:
  create: ask
  provider: github
  draft_for:
    - blocked
    - needs_intake
    - partial

changeset:
  semantic: true
  directory: ".harness/changesets"

cleanup:
  keep_failed_worktrees: true
  cleanup_after_reconcile: false
```

## 11. Git Ignore Requirements

Must ignore:

```gitignore
harness.db
harness.db-wal
harness.db-shm
.symphony/state.db
.symphony/worktrees/
.symphony/runs/*/harness.base.db
.symphony/runs/*/logs/
```

Must not ignore:

```gitignore
.harness/runs/*/SUMMARY.md
.harness/runs/*/RESULT.json
.harness/changesets/
```

## 12. Acceptance Criteria

### 12.1 MVP Acceptance

Given an eligible story exists in `harness.db`, when a user runs:

```bash
harness-symphony run <story-id> --prepare-only
```

then Symphony:

1. refuses to use the root checkout as the run workspace
2. creates a dedicated worktree
3. copies `harness.db`
4. records base DB metadata
5. writes `RUN_CONTRACT.json`
6. exports `HARNESS_DB_PATH` for the copied DB
7. leaves root `harness.db` unchanged

### 12.2 Agent Result Acceptance

Given an agent run finishes, Symphony accepts the run only if:

1. `SUMMARY.md` exists
2. `RESULT.json` exists
3. `RESULT.json` has a valid outcome
4. required validation evidence is present or explicitly marked unavailable
5. forbidden local runtime files are not staged for commit

### 12.3 PR Acceptance

Given PR creation is enabled, Symphony:

1. includes summary and result artifacts
2. includes semantic changeset only if generated
3. does not include `harness.db`
4. does not include `.symphony/state.db`
5. does not include worktree files or DB snapshots

### 12.4 Reconciliation Acceptance

Given a merged PR with a valid semantic changeset, Symphony:

1. verifies the PR is merged
2. verifies the changeset was not already applied
3. applies operations transactionally through `harness-cli`
4. marks the run reconciled
5. reports manual repair steps on conflict

## 13. Product Risks

### 13.1 Too Much Ceremony

Risk: a user sees Symphony as bureaucracy around simple coding tasks.

Mitigation:

- make `doctor`, `work list`, and `run --prepare-only` excellent
- keep PR creation optional
- default blocked/intake-only PRs to draft

### 13.2 Database State Confusion

Risk: users do not know whether `harness.db`, docs, or changesets are the source
of truth.

Mitigation:

- document committed artifacts as team truth
- make `harness.db` rebuildable
- make unreconciled PRs visible

### 13.3 Agent Adapter Lock-In

Risk: a Harness-native tool becomes Codex-only.

Mitigation:

- make Codex the first adapter, not the core model
- support custom command adapters in v1
- keep agent protocol file-based where possible

### 13.4 Changeset Complexity

Risk: semantic DB reconciliation consumes the whole project.

Mitigation:

- defer apply/rebuild until run contracts work
- start with append-only operations
- avoid raw row-level SQLite diffs

## 14. Recommended Implementation Order

1. Add `harness-symphony doctor`.
2. Add config loading and path normalization.
3. Add run state store.
4. Add worktree creation and copied DB wiring.
5. Add `RUN_CONTRACT.json`.
6. Add `RESULT.json` validation.
7. Add custom command agent adapter.
8. Add `runs list/show`.
9. Add optional PR creation.
10. Add semantic changeset generation.
11. Add manual reconciliation.
12. Add local queue.
13. Add auto-mode.
14. Add external work-source adapters.

## 15. One-Sentence Positioning

Harness Symphony is a safe agent workbench for turning Harness stories into
isolated, reviewable runs; it can become a Symphony-style autonomous
orchestrator only after that local loop is trusted.
