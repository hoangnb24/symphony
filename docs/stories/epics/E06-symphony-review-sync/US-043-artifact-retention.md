# US-043 Run Artifact Retention And Compaction

## Status

planned

## Lane

normal

## Product Contract

Symphony must keep permanent changeset history while giving users a safe way to
compact old run summaries and results.

## Relevant Product Docs

- `docs/SYMPHONY_SCOPE.md`

## Acceptance Criteria

- `.harness/changesets/` is never pruned by compaction.
- `.harness/runs/<run_id>/SUMMARY.md` and `RESULT.json` are kept by default.
- `harness-symphony runs compact --keep-last <n>` compacts or deletes old run
  artifacts according to documented policy.
- Compaction refuses unsafe values and reports exactly what it changed.
- `.symphony/` runtime state remains freely cleanable and ignored.

## Design Notes

- Command: `runs compact --keep-last <n>`.
- Compaction may archive old summaries or delete old run folders.
- Keep retention behavior deterministic.

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-043 --unit 1 --integration 1 --e2e 0 --platform 1`.

| Layer | Expected proof |
| --- | --- |
| Unit | Retention selection and unsafe-value tests. |
| Integration | Fixture run directories compact while changesets remain untouched. |
| E2E | n/a. |
| Platform | File-system smoke. |
| Release | `cargo test --workspace`; `cargo fmt --check`; `cargo clippy --workspace -- -D warnings`. |

## Harness Delta

Controls repository growth without weakening changeset source-of-truth history.

## Evidence

Add validation output after implementation.

