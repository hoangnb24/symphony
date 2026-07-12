#!/usr/bin/env bash
set -euo pipefail
archive=${1:?archive required}
list=$(mktemp); trap 'rm -f "$list"' EXIT
tar -tf "$archive" >"$list"
expected='^(\./)?(bin/harness-symphony(\.exe)?|share/harness-symphony/resource-manifest\.json|share/harness-symphony/web-ui/.+|LICENSE|release-metadata\.json|provenance\.json|sbom\.spdx\.json)$'
if rg -v "$expected" "$list"; then echo "unexpected artifact content" >&2; exit 1; fi
if rg -i 'harness\.db|harness-cli|crates/|target/|\.symphony/' "$list"; then echo "source or opaque state leaked" >&2; exit 1; fi
echo "Release artifact boundary passed"
