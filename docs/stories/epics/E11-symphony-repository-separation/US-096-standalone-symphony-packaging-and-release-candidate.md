# US-096 Standalone Symphony Packaging And Release Candidate

## Status

planned

## Owner Repository

`symphony`

## Lane

normal; publication remains gated by high-risk `US-100`.

## Product Contract

A Symphony release candidate turns the locator/minimal standalone layout proven
by `US-095` into reproducible production artifacts containing the backend
binary and Web assets. It runs outside a source checkout, has checksums, and can
be consumed without Cargo or a sibling repository-harness clone.

## Relevant Product Docs

- Target installation and release docs.
- Target `docs/contracts/harness-runtime-v1.md`.

## Acceptance Criteria

- A documented packaging decision builds on the executable-relative locator
  proven in `US-095` and chooses embedded Web assets or a stable production
  archive layout. No packaged binary returns 503 merely because the source tree
  is absent.
- If using an archive, its layout is stable and includes
  `bin/harness-symphony(.exe)`, `share/harness-symphony/web-ui/**`, LICENSE, and
  provenance/version metadata.
- Release builds exist for macOS arm64/x64, Linux x64/arm64, and Windows x64,
  with `.exe` where required and SHA-256 files for every artifact.
- A machine-readable release manifest records, for each artifact, target
  triple, archive name, binary path/name, Web asset root/hash, archive checksum,
  source SHA, Symphony version, and supported Harness protocol/schema range.
- The binary reports the Symphony version and supported Harness protocol range.
- An unpacked archive in a temporary directory passes `--version`, `doctor`,
  Web health, board API, and root UI checks against the `US-095` fixture.
- Packaged Electron `extraResources` and backend lookup support the platform
  binary including `.exe`, use the same resource manifest, and open a standard
  Harness repo that lacks Symphony source.
- Release workflow runs Rust, Web build, Playwright, desktop smoke, and
  standalone compatibility on native runners before upload. Each native
  artifact runs the Harness JSON contract smoke; Windows uses a PowerShell
  verifier rather than a Unix-only archive/checksum script.
- Package/npm identifiers and app metadata no longer use
  `@repository-harness` or the repository-harness URL.
- No artifact contains `harness-cli` source or an opaque target `harness.db`.
- Signing, notarization, and auto-update are explicitly deferred rather than
  silently claimed.
- The story produces a local/CI release candidate only; remote publication is
  authorized in `US-100` after Harness cleanup proof also passes.

## Design Notes

- Prefer one predictable resource locator relative to the executable, with an
  explicit override for development/tests.
- Release metadata records source commit, protocol version, and checksums.
- Keep CLI and desktop packaging separable so desktop signing does not block the
  first standalone CLI release.

## Validation

| Layer | Expected proof |
| --- | --- |
| Unit | Resource path selection and platform binary naming. |
| Integration | Archive unpack plus Web asset serving. |
| E2E | Packaged CLI operates on the pinned Harness fixture. |
| Platform | Native matrix artifacts and Electron smoke. |
| Release | Checksums verify and artifact contents match the manifest. |

```bash
scripts/build-release.sh
shasum -a 256 -c dist/*.sha256
tar -tf dist/harness-symphony-*.tar.gz
tests/compatibility/smoke-release-artifact.sh dist/<artifact> "$FIXTURE"
powershell -File tests/compatibility/smoke-release-artifact.ps1 -Artifact dist/<windows-artifact> -Fixture "$FIXTURE"
scripts/verify-release-manifest.sh dist/release-manifest.json
git diff --check
```

## Harness Delta

None. Symphony consumes the Harness release as an external compatibility
fixture.

## Evidence

Pending implementation. Record the machine-readable artifact matrix, checksums,
native Harness-contract/unpacked smoke logs, Electron resource lookup, and
deferred signing limitations.
