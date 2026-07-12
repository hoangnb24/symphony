# Symphony Quick Start

This guide is for an operator running a built Symphony artifact against a
Harness-enabled target repository. Packaged release downloads arrive in US-096;
for now, use a locally built artifact supplied from this source repository.

## 1. Choose the artifact and target repository

The artifact can live anywhere. Always identify the target with `--repo-root`,
which makes commands independent of the current working directory.

```bash
SYMPHONY=/absolute/path/to/harness-symphony
REPO=/absolute/path/to/target-repository
```

```powershell
$Symphony = "C:\absolute\path\to\harness-symphony.exe"
$Repo = "C:\absolute\path\to\target-repository"
```

The target needs a compatible Harness CLI and an initialized Harness database.
Symphony discovers the executable according to the
[`runtime contract`](contracts/harness-runtime-v1.md). Optional settings are
documented in [`examples/symphony.yml`](../examples/symphony.yml).

## 2. Check readiness and select work

```bash
"$SYMPHONY" --repo-root "$REPO" doctor
"$SYMPHONY" --repo-root "$REPO" work list
```

```powershell
& $Symphony --repo-root $Repo doctor
& $Symphony --repo-root $Repo work list
```

Fix `doctor` failures before a run. In `work list`, `yes` means runnable,
`warn` means non-runnable until an operator resolves the reported gap (commonly missing verification), and
`no` means Symphony will not run that story yet.

## 3. Prepare and execute

For a normal or high-risk story, inspect the isolated workspace and contract
before launch:

```bash
"$SYMPHONY" --repo-root "$REPO" run <story-id> --prepare-only
"$SYMPHONY" --repo-root "$REPO" run <story-id>
```

```powershell
& $Symphony --repo-root $Repo run <story-id> --prepare-only
& $Symphony --repo-root $Repo run <story-id>
```

Preparation creates a worktree below `.symphony/worktrees/<run_id>/` and writes
`.harness/runs/<run_id>/RUN_CONTRACT.json` inside that workspace. Harness
creates the isolated database through its WAL-safe snapshot protocol. Symphony
then launches the configured agent adapter.

Tiny-lane stories may run in the target checkout:

```bash
"$SYMPHONY" --repo-root "$REPO" run <story-id> --here
```

```powershell
& $Symphony --repo-root $Repo run <story-id> --here
```

Symphony refuses `--here` for normal and high-risk stories. The lightweight
path still uses an isolated database and requires the same result artifacts.

## 4. Understand the outputs

Every completed agent run must write, under its workspace:

```text
.harness/runs/<run_id>/SUMMARY.md
.harness/runs/<run_id>/RESULT.json
```

`RESULT.json` is the machine-readable outcome contract; `SUMMARY.md` is the
human review narrative. If the run performs durable Harness mutations, the
Harness CLI also writes:

```text
.harness/changesets/<run_id>.changeset.jsonl
```

The distinction matters:

- `SUMMARY.md`, `RESULT.json`, logs, and validation output are local run
  evidence. They are inspected by Symphony and used by the review UI, but are
  not durable repository records.
- Product/code/docs changes and semantic changesets are branch changes and may
  be committed and reviewed in a pull request.
- `harness.db` and `.symphony/state.db` are local indexes and are never PR
  artifacts.

For example, a run that changes `src/parser.rs` and records a story transition
can produce a branch containing `src/parser.rs` plus one changeset JSONL. Its
summary becomes the PR description; its `RESULT.json` remains local evidence.

Inspect a run with:

```bash
"$SYMPHONY" --repo-root "$REPO" status
"$SYMPHONY" --repo-root "$REPO" runs list
"$SYMPHONY" --repo-root "$REPO" runs show <run_id>
```

```powershell
& $Symphony --repo-root $Repo status
& $Symphony --repo-root $Repo runs list
& $Symphony --repo-root $Repo runs show <run_id>
```

## 5. Optional pull request and post-merge sync

When a PR provider is configured, Symphony can create or retry a PR from a
finished run:

```bash
"$SYMPHONY" --repo-root "$REPO" pr create <run_id>
"$SYMPHONY" --repo-root "$REPO" pr retry <run_id>
```

```powershell
& $Symphony --repo-root $Repo pr create <run_id>
& $Symphony --repo-root $Repo pr retry <run_id>
```

PR creation is optional. It uses the summary as the PR body and publishes the
run branch containing its product changes and durable semantic changeset; it
does not turn local result files or databases into committed state.

After the PR is accepted, pull the merged branch and replay new committed
changesets into the target's local Harness database:

```bash
"$SYMPHONY" --repo-root "$REPO" sync
```

```powershell
& $Symphony --repo-root $Repo sync
```

`sync` goes through the typed Harness changeset-status/apply protocol. It is
idempotent: an already applied changeset is skipped, while an invalid or
incompatible change fails before Symphony marks it applied.

## Contributor-only source workflow

Operators do not need Cargo, the source tree, or a repository-relative binary
path. Contributors who are building the current artifact use:

```bash
cargo build --locked -p harness-symphony
cargo test --workspace --locked
cargo run --locked -p harness-symphony -- --repo-root /path/to/target doctor
```

See the repository [`README`](../README.md) for the complete contributor and
Web UI validation commands.
