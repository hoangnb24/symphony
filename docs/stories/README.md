# Stories

Stories are work packets. They turn product intent into bounded implementation
and validation work.

Current runnable ownership lives in this repository's local Harness database.
The E11 target sequence (`US-094` through `US-096`) is the active standalone
product work. Imported E05–E08 and US-046 packets are historical implementation
evidence; their Markdown headings do not reactivate durable work. See
the [imported Symphony history](../history/README.md) for reviewed dispositions.

## Normal Story

Use `docs/templates/story.md` for normal feature work.

Suggested path:

```text
docs/stories/epics/E01-domain-name/US-001-short-story-title.md
```

## High-Risk Story

Use `docs/templates/high-risk-story/` when the feature intake classifies work as
high-risk.

Suggested path:

```text
docs/stories/epics/E02-risky-domain/US-012-risky-story-title/
  execplan.md
  overview.md
  design.md
  validation.md
```

## Status Flow

```text
planned -> in_progress -> implemented
                  |
                  v
               changed
                  |
                  v
               retired
```
