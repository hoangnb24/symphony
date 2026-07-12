# Optional Tooling

Symphony builds, runs, and validates without Impeccable or any project-local
design-tool configuration. Impeccable is an optional external provider for
design review; it is not a dependency, bundled extension, or prerequisite.

## Design Review Degrade Ladder

For the `design-review` capability:

1. **No provider registered:** skip the optional review cleanly. Record
   `design-review: inactive` when a trace is being written. This is not drift
   and must not fail product validation.
2. **Provider registered but missing or unusable:** continue with the required
   build, Playwright, accessibility, and human screenshot checks, but report a
   degraded warning and mark proof weak where the selected workflow requires
   the provider.
3. **Provider present and usable:** it may add an optional design audit or
   validation result. Its result supplements rather than replaces required
   executable and human review evidence.

The generic Harness tool registry owns provider discovery and status. Symphony
does not prescribe an Impeccable install command or scan path because those are
external/runtime-specific concerns. In particular, do not add `.impeccable`,
`.codex`, or `.agents` configuration to this repository.

The archived intake-griller under
[`archive/extensions/harness-intake-griller/`](archive/extensions/harness-intake-griller/)
is historical source material. It is deliberately outside hidden runtime
discovery paths and is neither executable nor required before a Symphony run.
