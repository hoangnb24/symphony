# Imported Symphony History

This directory indexes planning and delivery evidence imported from
`repository-harness`. These documents explain how the standalone Symphony
code arrived at its current behavior; they are not an active queue and packet
status text is not authoritative for current work.

## Provenance

- Source repository: `repository-harness`
- Reviewed source commit: `c9a64668d2723ecc1de4779a5df8e214493b5f6f`
- Import lineage: the filtered bootstrap recorded in
  [`../provenance/repository-harness-import.md`](../provenance/repository-harness-import.md)
- US-094 review: [`../provenance/e11-us094-history-review.md`](../provenance/e11-us094-history-review.md)

## Disposition Index

| Evidence | Historical disposition | Current-work meaning |
| --- | --- | --- |
| E05 | Delivered foundation history | Do not reactivate the epic from this README. |
| E06 | Delivered review/sync history | Do not treat old source-DB wording as the standalone runtime contract. |
| E07 | Mixed delivered and future planning history | Confirm current code and target Harness state before opening work. |
| E08 | Delivered and superseded controller history | Individual packet status may be stale; use the exceptions below. |
| US-046 | Implemented with an evidence caveat | Unit/integration proof exists, but the imported packet explicitly does not claim the original live E2E terminal-event proof. |
| US-061 | Retired | The proposed FrankenTUI surface is not part of the standalone product plan. |
| US-063 | Retired | The proposed completion alert is not part of the standalone product plan. |
| US-065 | Implemented despite stale `planned` packet text | Code and retained validation implement Codex lifecycle-based runtime behavior. |
| US-066 | Implemented despite stale `planned` packet text | Code and retained Web UI tests implement actionable Needs Attention details. |

Historical documents can still identify rationale, prior acceptance criteria,
or evidence gaps. They cannot by themselves make a story runnable. Current
work must be created or reconciled through the target repository's Harness
state.
