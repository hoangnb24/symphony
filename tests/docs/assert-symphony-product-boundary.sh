#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)

require_file() { [[ -f "$repo_root/$1" ]] || { echo "missing shipped file: $1" >&2; exit 1; }; }
reject_rg_match() {
  local description=$1 pattern=$2; shift 2
  local status
  set +e
  rg -n --glob '*.md' "$pattern" "$@"
  status=$?
  set -e
  case $status in
    0) echo "$description" >&2; exit 1 ;;
    1) return 0 ;;
    *) echo "rg failed while checking: $description" >&2; exit "$status" ;;
  esac
}

for file in README.md docs/SYMPHONY_QUICKSTART.md docs/SYMPHONY_SCOPE.md \
  docs/product/symphony-web-ui-controller.md docs/contracts/harness-runtime-v1.md \
  examples/symphony.yml docs/TOOL_REGISTRY.md docs/stories/backlog.md \
  docs/stories/US-046-first-class-symphony-codex-adapter.md \
  docs/history/README.md docs/provenance/e11-us094-history-review.md \
  docs/OPTIONAL_TOOLING.md; do
  require_file "$file"
done

hidden=$(git -C "$repo_root" ls-files -- '.agents/**' '.codex/**' '.impeccable/**')
[[ -z "$hidden" ]] || { echo "tracked project-local tool state is forbidden:" >&2; printf '%s\n' "$hidden" >&2; exit 1; }

for epic in E05-symphony-local-runner E06-symphony-review-sync E07-symphony-automation E08-symphony-web-ui-controller; do
  require_file "docs/stories/epics/$epic/README.md"
done

reject_rg_match "current operator docs still assume a repository-harness source checkout" \
  '(\.\./repository-harness|/repository-harness/|repository-harness/target/(debug|release)/harness-symphony)' \
  "$repo_root/README.md" "$repo_root/docs/SYMPHONY_QUICKSTART.md" "$repo_root/docs/SYMPHONY_SCOPE.md"
reject_rg_match "current operator docs require a project-local hidden tooling tree" \
  '(^|[ /`])\.(agents|codex|impeccable)/' \
  "$repo_root/README.md" "$repo_root/docs/SYMPHONY_QUICKSTART.md" "$repo_root/docs/SYMPHONY_SCOPE.md"

rg -q 'historical|completed' "$repo_root/docs/stories/epics/E05-symphony-local-runner" \
  || { echo "E05 history is not labelled historical/completed" >&2; exit 1; }
for item in '#10' '#11' '#12' '#14'; do
  rg -Fq "$item" "$repo_root/docs/provenance/e11-us094-history-review.md" \
    || { echo "reviewed disposition for backlog item $item is missing" >&2; exit 1; }
done
rg -qi 'optional|clean skip|absen' "$repo_root/docs/OPTIONAL_TOOLING.md" "$repo_root/docs/TOOL_REGISTRY.md" \
  || { echo "optional external design tooling behavior is undocumented" >&2; exit 1; }

echo "Symphony product boundary checks passed"
