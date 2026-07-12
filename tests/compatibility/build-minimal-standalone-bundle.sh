#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
binary=${1:?usage: build-minimal-standalone-bundle.sh BINARY DESTINATION}
destination=${2:?usage: build-minimal-standalone-bundle.sh BINARY DESTINATION}

[[ -x "$binary" ]] || { echo "missing executable Symphony binary: $binary" >&2; exit 1; }
[[ -f "$repo_root/crates/harness-symphony/web-ui/dist/index.html" ]] || {
  echo "missing built Web UI; run npm --prefix crates/harness-symphony/web-ui run build" >&2
  exit 1
}

rm -rf "$destination"
mkdir -p "$destination/web-ui-dist"
cp "$binary" "$destination/$(basename "$binary")"
cp -R "$repo_root/crates/harness-symphony/web-ui/dist/." "$destination/web-ui-dist/"

test -x "$destination/$(basename "$binary")"
test -f "$destination/web-ui-dist/index.html"
echo "$destination/$(basename "$binary")"
