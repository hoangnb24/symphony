#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

[[ ${E11_US094_FORCE_FAILURE:-0} != 1 ]] || {
  echo "intentional US-094 negative verification fixture" >&2
  exit 1
}

cargo fmt --manifest-path "$repo_root/Cargo.toml" --all --check
cargo test --manifest-path "$repo_root/Cargo.toml" -p harness-symphony --locked \
  config::tests::shipped_example_parses_and_resolves_all_relative_paths_from_repo_root -- --exact
cargo build --manifest-path "$repo_root/Cargo.toml" -p harness-symphony --release --locked
"$repo_root/tests/docs/assert-symphony-product-boundary.sh"
node "$repo_root/tests/docs/check-markdown-links.mjs" "$repo_root"
node "$repo_root/tests/docs/check-operator-command-examples.mjs" "$repo_root"
SYMPHONY_RELEASE_BIN="$repo_root/target/release/harness-symphony" \
  "$repo_root/tests/docs/test-operator-outside-source.sh"
git -C "$repo_root" diff --check

echo "US-094 product-doc and optional-tooling verification passed"
