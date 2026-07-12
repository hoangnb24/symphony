#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/crates/harness-symphony"
printf '%s\n' '[workspace]' 'members = ["crates/harness-cli", "crates/harness-symphony"]' >"$tmp/Cargo.toml"
printf '%s\n' 'name = "harness-cli"' >"$tmp/Cargo.lock"

if US091_ROOT="$tmp" "$root/tests/standalone/verify-workspace.sh" >/dev/null 2>&1; then
  echo "standalone verifier accepted a workspace containing harness-cli" >&2
  exit 1
fi

echo "US-091 standalone verifier negative fixture passed"
