# E06 Symphony Review And Sync

## Goal

Turn local runs into reviewable committed artifacts and make merged changesets
safe to apply on any clone.

## Product Contract

Run artifacts and changesets are committed review surfaces. Root `harness.db`
is updated after merge by idempotent sync or rebuild, never by committing the DB.

## Stories

1. `US-040` - Changeset rendering in summaries.
2. `US-041` - Optional PR creation.
3. `US-042` - `harness-symphony sync`.
4. `US-043` - Artifact retention and compaction.

## Exit Criteria

- PRs include `SUMMARY.md`, `RESULT.json`, and semantic changesets.
- Reviewers see a human-readable Harness changes table.
- `sync` applies committed-but-unapplied changesets safely and is a no-op when
  repeated.
- `.harness/changesets/` is permanent; `.harness/runs/` can be compacted.

