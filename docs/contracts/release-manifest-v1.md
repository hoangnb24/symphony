# Symphony Release Manifest v1

`dist/release-manifest.json` is the machine-readable index for a local or CI
release candidate.

Top-level fields:

- `manifest_version`: integer `1`.
- `product`: `harness-symphony`.
- `symphony_version` and `source_sha`.
- `source_dirty`: `false` for an aggregate candidate; test-only native
  manifests may truthfully record `true`.
- `supported_harness`: protocol `1`, schema range `1..13`, supported current
  database schemas `12..13`.
- `artifacts`: one entry per native archive.

Every artifact entry records `target_triple`, `archive_name`, `archive_format`,
`binary_path`, `web_asset_root`, `web_asset_sha256`, `archive_sha256`,
`metadata_sha256`, `provenance_sha256`, and `sbom_sha256`.

All paths are relative archive paths and must be safe: no absolute path, empty
segment, or `..` segment. A verifier must recompute every checksum, reject
duplicate archive entries, reject opaque databases or Harness CLI source and
binaries, and compare the internal metadata to the manifest rather than
trusting producer output.

Native jobs emit single-entry manifests. The final aggregation job merges them
only when every top-level identity field matches and target triples and archive
names are unique.

Verification mode is explicit: `--native` accepts exactly one supported target
triple, while `--aggregate` requires exactly all five supported triples and a
clean source: `aarch64-apple-darwin`, `x86_64-apple-darwin`,
`aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu`, and
`x86_64-pc-windows-msvc`.
