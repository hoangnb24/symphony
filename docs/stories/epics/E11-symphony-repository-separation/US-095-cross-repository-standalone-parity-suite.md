# US-095 Cross-Repository Standalone Parity Suite

## Status

planned

## Owner Repository

`symphony`

## Lane

normal with strong integration, E2E, and platform proof.

## Product Contract

Symphony is standalone only when a build from its own repository can operate on
a separately created Harness project without either source checkout on its
runtime path. This story creates that proof before source deletion is allowed.

## Dependencies

- `US-094` completed in the target; therefore the protocol adapter and current
  operator/contract docs are both in place.

## Relevant Product Docs

- Target `docs/contracts/harness-runtime-v1.md`
- Target Quickstart
- E11 gates B-D

## Acceptance Criteria

- A deterministic script creates a temporary Git repository, installs Harness
  template files and a checksum-verified CLI from the same exact `US-092`
  release/tag (forcing an older local CLI to upgrade), verifies the complete
  protocol tuple before initialization, initializes its database, adds
  `US-INDEP-001`, and configures a fixture agent with PR creation disabled.
- The fixture does not copy `crates/harness-cli` and does not reference a
  sibling repository-harness checkout after bootstrap.
- A release-mode Symphony binary is invoked from a third temporary directory
  with `--repo-root <fixture>`.
- `doctor`, `work list`, and `run US-INDEP-001 --prepare-only` pass.
- Prepare-only leaves the root canonical logical-state hash unchanged and
  creates only documented local run/worktree state; its isolated DB includes a
  deliberately uncheckpointed WAL commit through the snapshot protocol.
- A complete deterministic fixture-agent run produces valid `SUMMARY.md`,
  `RESULT.json`, app/run evidence, and one scoped semantic changeset.
- The run branch is reviewed and merged locally with a deterministic merge
  script. The first sync applies exactly the expected scoped changeset; the
  second sync reports a no-op, and logical DB plus active changeset hashes are
  unchanged by the second call.
- Web `/health`, `/api/board`, and `/` serve correct JSON/UI assets from the
  standalone artifact. This story implements the minimal stable
  executable-relative resource locator and produces a test-only
  binary-plus-Web bundle in a third directory, so `/` passes before production
  packaging work in `US-096`.
- Existing Web UI build, 19 Playwright tests, and desktop smoke pass.
- Electron root detection uses normal Harness evidence (compatible CLI/config
  and database state), not `crates/harness-symphony`. Desktop smoke opens the
  explicit external fixture and rejects a directory that only happens to have
  the old crate path.
- Old, malformed, and missing-required-capability Harness fixtures hard-fail
  before the root DB or active changeset set changes.
- A selected but missing execution agent fails run setup before execution.
  An absent unregistered optional provider clean-skips; a registered-but-missing
  optional provider reports degraded/weak proof without blocking doctor, work
  listing, or prepare-only behavior.
- Linux, macOS, and Windows CI exercise CLI discovery and fixture behavior.
- No test requires live Codex or GitHub mutation; separate optional live smoke
  evidence may supplement but not replace deterministic proof.
- Repository-harness still contains the source implementation throughout this
  story.

## Design Notes

- Harness positive pin: use the exact protocol-v1 release produced by `US-092`.
  Keep `harness-cli-v0.1.11`/schema 12 as a legacy negative fixture that must
  fail before mutation.
- Fixture agent: writes exact required artifacts and uses the run environment.
- Comparison: preserve golden board/run results from the frozen baseline where
  behavior is expected to remain identical.
- Isolation: create fresh fixture per mutation scenario.
- Web resource boundary: `US-095` proves the runtime locator and a minimal
  test-only standalone layout; `US-096` owns reproducible multi-platform
  archives, metadata, checksums, and published release workflow.

## Validation

| Layer | Expected proof |
| --- | --- |
| Unit | Protocol adapter and artifact validators. |
| Integration | Pinned Harness install, story graph, prepare, execute, changeset apply, sync. |
| E2E | Web/Playwright and complete fixture-agent workflow. |
| Platform | macOS/Linux/Windows CLI discovery plus Electron smoke. |
| Release | Release build runs outside both source trees. |

```bash
cargo build --release --locked
tests/compatibility/bootstrap-harness-fixture.sh --harness-ref "$HARNESS_PROTOCOL_V1_TAG" --upgrade-cli --story US-INDEP-001 "$FIXTURE"
tests/compatibility/assert-contract-tuple.sh "$FIXTURE" "$HARNESS_PROTOCOL_V1_TAG"
(cd "$FIXTURE" && scripts/bin/harness-cli db snapshot --output "$SNAPSHOT" --json)
(cd "$(mktemp -d)" && "$BIN" --repo-root "$FIXTURE" doctor)
(cd "$(mktemp -d)" && "$BIN" --repo-root "$FIXTURE" work list)
"$BIN" --repo-root "$FIXTURE" run US-INDEP-001 --prepare-only
tests/compatibility/run-fixture-agent.sh "$BIN" "$FIXTURE" US-INDEP-001
tests/compatibility/review-and-merge-fixture-run.sh "$FIXTURE" US-INDEP-001
tests/compatibility/assert-sync.sh --expect applied "$BIN" "$FIXTURE" US-INDEP-001
tests/compatibility/assert-sync.sh --expect no-op --assert-state-unchanged "$BIN" "$FIXTURE" US-INDEP-001
tests/compatibility/build-minimal-standalone-bundle.sh "$BIN" "$STANDALONE"
tests/compatibility/smoke-standalone-web.sh "$STANDALONE" "$FIXTURE"
npm --prefix crates/harness-symphony/web-ui run build
npm --prefix crates/harness-symphony/web-ui run e2e
npm --prefix crates/harness-symphony/web-ui run desktop:smoke -- --repo-root "$FIXTURE"
tests/compatibility/assert-upgrade-required.sh --harness-tag harness-cli-v0.1.11
git diff --check
```

`smoke-standalone-web.sh` starts the bundle on an ephemeral port, waits for and
parses readiness, asserts `/health`, `/api/board`, and `/` plus referenced UI
assets, terminates the complete process tree on success/failure, and fails
closed on timeout or cleanup failure.

## Harness Delta

None in the fixture. The suite consumes released Harness behavior and must not
patch it locally to make tests pass.

## Evidence

Pending implementation. Attach fixture provenance, before/after checksums,
artifact paths, protocol verdicts, resource-locator proof, local review/merge
SHA, first/second sync JSON plus logical/hash comparison, and desktop fixture
identity.
