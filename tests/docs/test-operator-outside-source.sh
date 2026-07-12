#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
symphony=${SYMPHONY_RELEASE_BIN:-"$repo_root/target/release/harness-symphony"}
harness_cli="$repo_root/scripts/bin/harness-cli"
[[ -x "$symphony" ]] || { echo "missing release Symphony binary: $symphony" >&2; exit 1; }
[[ -x "$harness_cli" ]] || { echo "missing local Harness CLI: $harness_cli" >&2; exit 1; }
[[ "$($harness_cli --version)" == "harness-cli 0.1.14" ]] || { echo "operator test requires exact local harness-cli 0.1.14" >&2; exit 1; }

tmp=$(mktemp -d "${TMPDIR:-/tmp}/symphony operator.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
fixture="$tmp/product repo with spaces"
install="$tmp/release install"
mkdir -p "$fixture/.harness" "$fixture/scripts/bin" "$fixture/scripts/schema" "$install"
cp "$symphony" "$install/harness-symphony"
cp "$harness_cli" "$fixture/scripts/bin/harness-cli"
cp "$repo_root"/scripts/schema/*.sql "$fixture/scripts/schema/"
cp "$repo_root/examples/symphony.yml" "$fixture/.harness/symphony.yml"
chmod +x "$install/harness-symphony" "$fixture/scripts/bin/harness-cli"
git -C "$fixture" init -q
git -C "$fixture" config user.name "Symphony operator test"
git -C "$fixture" config user.email "operator@example.invalid"
printf '.harness/runs/\n.symphony/\nharness.db\nharness.db-wal\nharness.db-shm\n' >"$fixture/.gitignore"
git -C "$fixture" add .gitignore .harness/symphony.yml scripts/bin/harness-cli scripts/schema
git -C "$fixture" commit -qm fixture

HARNESS_REPO_ROOT="$fixture" HARNESS_DB_PATH="$fixture/harness.db" "$fixture/scripts/bin/harness-cli" init >/dev/null
HARNESS_REPO_ROOT="$fixture" HARNESS_DB_PATH="$fixture/harness.db" "$fixture/scripts/bin/harness-cli" \
  story add --id US-OPERATOR --title "Operator fixture" --lane tiny >/dev/null

before=$(git -C "$repo_root" ls-files -s README.md docs examples/symphony.yml | shasum -a 256 | awk '{print $1}')
"$install/harness-symphony" --repo-root "$fixture" config show >"$tmp/config-show.txt"
if ! "$install/harness-symphony" --repo-root "$fixture" doctor >"$tmp/doctor.txt" 2>&1; then
  cat "$tmp/doctor.txt" >&2
  echo "outside-source doctor failed" >&2
  exit 1
fi
"$install/harness-symphony" --repo-root "$fixture" work list >"$tmp/work-list.txt"
after=$(git -C "$repo_root" ls-files -s README.md docs examples/symphony.yml | shasum -a 256 | awk '{print $1}')
[[ "$before" == "$after" ]] || { echo "operator commands changed logical shipped-document identities" >&2; exit 1; }
rg -Fq 'scripts/bin/harness-cli' "$tmp/config-show.txt"
rg -q 'US-OPERATOR|Operator fixture' "$tmp/work-list.txt"

rm "$fixture/scripts/bin/harness-cli"
if "$install/harness-symphony" --repo-root "$fixture" work list >"$tmp/missing-cli.txt" 2>&1; then
  echo "work list unexpectedly accepted a missing configured Harness CLI" >&2; exit 1
fi
cp "$harness_cli" "$fixture/scripts/bin/harness-cli"; chmod +x "$fixture/scripts/bin/harness-cli"
printf 'version: [\n' >"$fixture/.harness/symphony.yml"
if "$install/harness-symphony" --repo-root "$fixture" config show >"$tmp/invalid-yaml.txt" 2>&1; then
  echo "config show unexpectedly accepted invalid YAML" >&2; exit 1
fi
rg -q 'config parse failed.*\.harness/symphony.yml' "$tmp/invalid-yaml.txt"

echo "outside-source operator checks passed"
