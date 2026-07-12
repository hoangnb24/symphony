#!/usr/bin/env bash
set -euo pipefail
output=${1:?usage: merge-release-manifests.sh OUTPUT INPUT...}; shift
[[ $# -gt 0 ]] || { echo "at least one native manifest is required" >&2; exit 2; }
root=$(cd "$(dirname "$0")/.." && pwd)
node "$root/scripts/release-metadata.mjs" merge-manifests "$output" "$@"
echo "Merged $# native release manifest(s): $output"
