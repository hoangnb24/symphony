# Symphony Harness Runtime Contract v1

Symphony is an independent consumer of the public Harness CLI protocol. It
does not link Harness source, inspect Harness tables, copy a live database, or
parse terminal prose.

## Pinned compatibility tuple

The first standalone Symphony release supports exactly this tested tuple:

| Field | Required value |
| --- | --- |
| Harness release | `harness-cli-v0.1.14` |
| CLI version | `0.1.14` |
| Protocol | `1` |
| CLI schema range | `1..=13` |
| Supported current database schema | `12..=13` |
| Symphony config | `1` |
| Run contract | `1` |
| Result contract | `1` |
| Required environment | `HARNESS_DB_PATH` |

The required capabilities are `stories.read.v1`, `stories.write.v1`,
`work-graph.read.v1`, `story-dependencies.read-write.v1`,
`story-hierarchy.read-write.v1`, `changesets.apply.v1`,
`changesets.status-sha.v1`, `isolated-db.v1`,
`isolated-db-snapshot.v1`, and `semantic-operation-log.v1`.

CLI `0.1.11` with schema 12 is retained only as the legacy negative fixture.
It is not a supported protocol-v1 runtime.

## Executable discovery

Symphony resolves one executable in this order:

1. `repo.harness_cli` in `.harness/symphony.yml`.
2. `HARNESS_CLI_PATH`.
3. `scripts/bin/harness-cli` on macOS/Linux or
   `scripts/bin/harness-cli.exe` on Windows under the selected repository.
4. `harness-cli`/`harness-cli.exe` on `PATH`.

Configured relative paths are resolved from `repo.root`. Paths are passed as
an executable plus argument array; spaces and `.exe` suffixes never require
shell quoting.

```yaml
version: 1
repo:
  root: .
  harness_db: harness.db
  harness_cli: tools/harness-cli
```

## Invocation boundary

Every protocol process receives an explicit working directory,
`HARNESS_REPO_ROOT`, and `HARNESS_DB_PATH`. Run-scoped writes additionally
receive `HARNESS_RUN_ID` and `HARNESS_RUN_MODE`. Reads time out after 30
seconds, mutations after 300 seconds, and combined stdout/stderr is capped at
16 MiB.

Machine output must be exactly one newline-terminated protocol-v1 JSON
envelope. Unknown additive fields are tolerated. A malformed envelope,
operation mismatch, unsupported version/range, missing capability, timeout,
output overflow, or undocumented exit/error pairing fails closed.

## Data access and mutation

- Work, dependency, and hierarchy state comes from one revisioned
  `query work-graph --json` call.
- Isolated run databases come from `db snapshot --json`, which uses SQLite's
  online backup protocol and includes committed WAL pages.
- Story status changes use compare-and-set `story update --json` operations.
- Changeset inspection and application use `db changeset status/apply --json`.
- Only `.symphony/state.db` remains directly owned through SQLite by Symphony.

Before a run, sync, selector, or Web mutation, Symphony discovers and validates
the runtime contract. Failure occurs before the Harness database changes.

## Upgrade and recovery

Install or replace the CLI through the checksum-verified immutable release:

```bash
curl -fsSL https://raw.githubusercontent.com/hoangnb24/repository-harness/harness-cli-v0.1.14/scripts/install-harness.sh \
  | bash -s -- --directory /absolute/repository/path --merge --upgrade-cli \
      --ref harness-cli-v0.1.14 --yes
```

PowerShell uses the same tag with `-Merge -UpgradeCli -Ref
harness-cli-v0.1.14 -Yes`. Contract discovery is read-only and may run while
the database is missing; database initialization remains an explicit operator
action.
