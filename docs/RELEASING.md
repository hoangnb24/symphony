# Building a Symphony Release Candidate

The five-platform workflow creates immutable candidates. Publishing a GitHub
release or tag remains a separate explicit owner-approved cutover action. The
published `symphony-v0.1.0` release is the initial baseline; later releases
must repeat the complete native and aggregate gates.

Prerequisites are the pinned Rust and Node versions, `npm ci`, and common Unix
archive tools. From the repository root:

```bash
npm --prefix crates/harness-symphony/web-ui ci
scripts/build-release.sh
scripts/verify-release-manifest.sh --native dist/release-manifest.json
```

The build creates one native archive, its `.sha256` sidecar, and
`release-manifest.json`. CI runs the same command once per target and later
aggregates the verified native entries.

Packaging rejects dirty release inputs and a `SOURCE_SHA` that differs from
checked-out `HEAD`. `SYMPHONY_RELEASE_ALLOW_DIRTY_TEST_ONLY=1` exists only so
the pre-commit story verifier can exercise packaging; it marks metadata and
provenance dirty and must never be set by release CI.

Unpack an archive and run its binary with an external Harness project:

```bash
tar -xzf dist/harness-symphony-<version>-<target>.tar.gz -C /tmp/symphony
/tmp/symphony/bin/harness-symphony --version
/tmp/symphony/bin/harness-symphony --repo-root /path/to/harness-project doctor
```

The Web UI is served from `share/harness-symphony/web-ui`; Cargo, npm, and a
Symphony source checkout are not runtime requirements.

Limitations: artifacts are unsigned. Notarization, code signing, installers,
auto-update, and remote publication are explicitly deferred to later work.
