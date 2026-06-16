# US-045 Auto-Mode Queue And Work Sources

## Status

planned

## Lane

normal

## Product Contract

After local isolated runs are proven, Symphony may support unattended work
polling through a queue and work-source adapters.

## Relevant Product Docs

- `docs/SYMPHONY_SCOPE.md`

## Acceptance Criteria

- Auto-mode is explicitly opt-in.
- Queue and retry semantics are introduced only for unattended or concurrent
  work.
- `HarnessDbWorkSource` is the first work source.
- External sources such as GitHub Issues, Linear, Jira, and remote Harness are
  adapter boundaries, not changes to run contracts.
- Existing run contract, result, changeset, and sync semantics are reused.

## Design Notes

- This is v3 work and should not start before v1/v2 exit criteria pass.
- Potential adapters: `HarnessDbWorkSource`, `GitHubIssueWorkSource`,
  `LinearWorkSource`, `JiraWorkSource`, `RemoteHarnessWorkSource`.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-045 --unit 1 --integration 1 --e2e 1 --platform 1`.

| Layer | Expected proof |
| --- | --- |
| Unit | Queue eligibility, retry, and adapter contract tests. |
| Integration | Harness DB work source feeds one queued run. |
| E2E | Opt-in auto-mode processes a fixture story through the existing runner. |
| Platform | Long-running process smoke with graceful shutdown. |
| Release | `cargo test --workspace`; `cargo fmt --check`; `cargo clippy --workspace -- -D warnings`. |

## Harness Delta

This is the first step toward Symphony-style automation, deliberately sequenced
after the local workbench.

## Evidence

Add validation output after implementation.

