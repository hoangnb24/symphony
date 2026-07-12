#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 --story ID [--upgrade-cli] [--template-root PATH] FIXTURE" >&2
  exit 2
}

story=""
upgrade=0
template_root=""
while [[ $# -gt 1 ]]; do
  case "$1" in
    --story) story=${2:?}; shift 2 ;;
    --upgrade-cli) upgrade=1; shift ;;
    --template-root) template_root=${2:?}; shift 2 ;;
    *) usage ;;
  esac
done
[[ $# -eq 1 && -n "$story" ]] || usage
fixture=$(mkdir -p "$1" && cd "$1" && pwd)
[[ -z "$(find "$fixture" -mindepth 1 -maxdepth 1 -print -quit)" ]] || {
  echo "fixture must be empty: $fixture" >&2; exit 1;
}

case "$(uname -s):$(uname -m)" in
  Darwin:arm64) asset=harness-cli-macos-arm64 ;;
  Darwin:x86_64) asset=harness-cli-macos-x64 ;;
  Linux:aarch64|Linux:arm64) asset=harness-cli-linux-arm64 ;;
  Linux:x86_64) asset=harness-cli-linux-x64 ;;
  MINGW*:*|MSYS*:*|CYGWIN*:*) asset=harness-cli-windows-x64.exe ;;
  *) echo "unsupported fixture platform: $(uname -s) $(uname -m)" >&2; exit 1 ;;
esac

download_release() {
  local version=$1 destination=$2 base checksum expected actual
  base="https://github.com/hoangnb24/repository-harness/releases/download/harness-cli-v${version}"
  curl --fail --silent --show-error --location "$base/$asset" --output "$destination"
  curl --fail --silent --show-error --location "$base/$asset.sha256" --output "$destination.sha256"
  expected=$(awk '{print $1}' "$destination.sha256")
  if command -v sha256sum >/dev/null 2>&1; then actual=$(sha256sum "$destination" | awk '{print $1}'); else actual=$(shasum -a 256 "$destination" | awk '{print $1}'); fi
  [[ "$actual" == "$expected" ]] || { echo "checksum mismatch for $asset v$version" >&2; exit 1; }
  chmod +x "$destination"
}

mkdir -p "$fixture/scripts/bin" "$fixture/scripts/schema" "$fixture/.harness"
if [[ -z "$template_root" ]]; then
  template_root=$(cd "$(dirname "$0")/../.." && pwd)
fi
# Copy only repository-level Harness inputs. The resulting fixture has no
# Symphony or harness-cli source tree and does not retain this source path.
cp "$template_root/AGENTS.md" "$fixture/AGENTS.md"
cp "$template_root/.gitignore" "$fixture/.gitignore"
cp -R "$template_root/scripts/schema/." "$fixture/scripts/schema/"

cli="$fixture/scripts/bin/harness-cli"
[[ "$asset" == *.exe ]] && cli="$fixture/scripts/bin/harness-cli.exe"
if [[ $upgrade -eq 1 ]]; then
  download_release 0.1.11 "$cli"
  [[ "$($cli --version)" == "harness-cli 0.1.11" ]]
fi
download_release 0.1.14 "$cli"
[[ "$($cli --version)" == "harness-cli 0.1.14" ]]

cat >"$fixture/.harness/symphony.yml" <<EOF
version: 1
repo:
  root: "."
  harness_db: "harness.db"
  harness_cli: "${cli#"$fixture/"}"
agent:
  adapter: "custom"
  command: ["bash", ".harness/fixture-agent.sh", "$story", "$cli"]
pull_request:
  create: "disabled"
changeset:
  directory: ".harness/changesets"
EOF
cp "$(dirname "$0")/run-fixture-agent.sh" "$fixture/.harness/fixture-agent.sh"
chmod +x "$fixture/.harness/fixture-agent.sh"

(cd "$fixture" && "$cli" query contract --json) >"$fixture/.harness/bootstrap-contract.json"
jq -e '.result.protocol_version == 1 and .result.cli_version == "0.1.14" and .result.schema_minimum == 1 and .result.schema_maximum == 13 and .result.database_state == "missing"' "$fixture/.harness/bootstrap-contract.json" >/dev/null
(cd "$fixture" && "$cli" init)
(cd "$fixture" && "$cli" story add --id "$story" --title "Independent fixture story" --lane normal --verify true)

cat >>"$fixture/.gitignore" <<'EOF'
/.symphony/
/harness.db
/harness.db-wal
/harness.db-shm
/.harness/runs/
EOF
(cd "$fixture" && git init -q && git config user.name "US-095 Fixture" && git config user.email "us095@example.invalid" && git add . && git commit -q -m "test: bootstrap independent Harness fixture")

[[ ! -e "$fixture/crates/harness-cli" && ! -e "$fixture/crates/harness-symphony" ]]
if rg -n '/repository-harness|/symphony|Documents/personal' "$fixture" --glob '!scripts/bin/harness-cli*' --glob '!.git/**'; then
  echo "fixture retained a source-checkout path" >&2; exit 1
fi
printf '%s\n' "$fixture"
