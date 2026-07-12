# 0008: Stable Symphony Release Layout

## Status

Accepted for local and CI release candidates. Remote publication remains gated
by US-100.

## Decision

Symphony uses a stable archive layout:

```text
bin/harness-symphony[.exe]
share/harness-symphony/web-ui/**
LICENSE
release-metadata.json
provenance.json
sbom.spdx.json
```

The backend locates Web assets relative to its executable at
`../share/harness-symphony/web-ui`. Development overrides and the US-095
executable-adjacent test layout remain explicit fallbacks.

The archive cannot contain its own checksum without creating a circular hash.
Therefore `release-metadata.json`, provenance, and the SBOM describe the source
and staged content; the external `release-manifest.json` and `.sha256` sidecar
bind those files to the final archive bytes.

CLI archives and Electron packages remain separable. Electron consumes the
same resource layout and metadata, but desktop signing must not block the CLI
release candidate.

## Reproducibility

Archive entries are sorted and assigned the source commit timestamp, numeric
owner/group zero, and normalized modes. Gzip timestamps are suppressed. Two
packages built from the same already-built native binary and Web output must be
byte-identical.

## Deferred Work

Code signing, notarization, installer distribution, auto-update, and remote
GitHub Release publication are not part of US-096.
