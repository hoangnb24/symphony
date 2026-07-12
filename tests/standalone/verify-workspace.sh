#!/usr/bin/env bash
set -euo pipefail

root=${US091_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}
cd "$root"

metadata=$(mktemp)
desktop_fixture=$(mktemp -d)
trap 'rm -f "$metadata"; rm -rf "$desktop_fixture"' EXIT
cargo metadata --locked --no-deps --format-version 1 >"$metadata"
jq -e '
  (.workspace_members | length) == 1 and
  (.packages | length) == 1 and
  .packages[0].name == "harness-symphony" and
  .packages[0].edition == "2021" and
  .packages[0].license == "MIT" and
  .packages[0].repository == "https://github.com/hoangnb24/symphony"
' "$metadata" >/dev/null

test "$(rg -c '^members = \["crates/harness-symphony"\]$' Cargo.toml)" = 1
rg -q '^name = "harness-symphony"$' Cargo.lock
if rg -q '^name = "harness-cli"$' Cargo.lock; then
  echo "standalone Cargo.lock contains harness-cli" >&2
  exit 1
fi
if rg -n '(/|[.][.]/)repository-harness' \
    --glob 'Cargo.toml' --glob 'package.json' --glob 'package-lock.json' \
    --glob '*.sh' --glob '*.ps1' --glob '*.cjs' --glob '*.ts' \
    --glob '!tests/**' \
    --glob '!scripts/verify-e11-us095.sh' .; then
  echo "live manifest or script names a sibling repository-harness checkout" >&2
  exit 1
fi

test -f README.md
test -f LICENSE
test -f .gitignore
test "$(cat .node-version)" = "24.9.0"
rg -q '^channel = "1\.92\.0"$' rust-toolchain.toml
rg -q 'playwright install --with-deps chromium' .github/workflows/standalone.yml
rg -q 'windows-2025' .github/workflows/standalone.yml
rg -q 'macos-14' .github/workflows/standalone.yml
rg -q 'ubuntu-24.04' .github/workflows/standalone.yml

cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
npm --prefix crates/harness-symphony/web-ui ci
npm --prefix crates/harness-symphony/web-ui run build
npm --prefix crates/harness-symphony/web-ui run e2e
tests/compatibility/bootstrap-harness-fixture.sh --upgrade-cli \
  --story US-STANDALONE-DESKTOP "$desktop_fixture"
npm --prefix crates/harness-symphony/web-ui run desktop:smoke -- \
  --repo-root "$desktop_fixture"
git diff --check

echo "US-091 standalone workspace verification passed"
