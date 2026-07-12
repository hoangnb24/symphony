#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
expected_head=""
expected_tag=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --expected-head)
      expected_head="${2:?missing --expected-head value}"
      shift 2
      ;;
    --expected-tag)
      expected_tag="${2:?missing --expected-tag value}"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

[[ -n "$expected_head" && -n "$expected_tag" ]]
test "$(git -C "$repo_root" branch --show-current)" = "$expected_head"
test "$(git -C "$repo_root" rev-parse "refs/tags/${expected_tag}^{commit}")" = "$(git -C "$repo_root" rev-parse HEAD)"

refs="$(git -C "$repo_root" for-each-ref --format='%(refname)' | LC_ALL=C sort)"
expected_refs="$(printf '%s\n%s' "refs/heads/$expected_head" "refs/tags/$expected_tag")"
test "$refs" = "$expected_refs"

objects="$(mktemp)"
paths="$(mktemp)"
trap 'rm -f "$objects" "$paths"' EXIT
git -C "$repo_root" rev-list --objects "refs/heads/$expected_head" "refs/tags/$expected_tag" >"$objects"
awk 'NF > 1 {$1=""; sub(/^ /, ""); print}' "$objects" | LC_ALL=C sort -u >"$paths"

if rg -n '(^|/)(harness\.db([-.].*)?|[^/]*\.(db|sqlite)([-.].*)?)$|^crates/harness-cli/|^scripts/schema/|^\.harness/changesets/|^\.agents/|^\.codex/|^\.impeccable/|^Cargo\.lock$' "$paths"; then
  echo "forbidden Harness/runtime path exists in reachable history" >&2
  exit 1
fi

rg -qx 'crates/harness-symphony/src/main.rs' "$paths"
rg -qx 'crates/harness-symphony/web-ui/src/main.tsx' "$paths"
rg -qx 'docs/SYMPHONY_SCOPE.md' "$paths"

manifest="$repo_root/docs/provenance/e11-filter-paths.txt"
commit_map="$repo_root/docs/provenance/e11-filter-repo-commit-map.txt"
test "$(wc -l <"$manifest" | tr -d ' ')" = "100"
test "$(LC_ALL=C sort "$manifest" | uniq -d | wc -l | tr -d ' ')" = "0"

for source_sha in \
  e7a124b763d5ab9dc6a9e6edfb5afff0867a7353 \
  444d793f17f3a4f59161a0bd15ec590c500c0150 \
  f539f5ded7479ffc932555977282c9c22e432746 \
  6e8243f2a5cb6a32cf0a7a0ecebdb257a429bdd9; do
  mapped="$(awk -v source="$source_sha" '$1==source {print $2}' "$commit_map")"
  [[ "$mapped" =~ ^[0-9a-f]{40}$ ]]
  test "$mapped" != "0000000000000000000000000000000000000000"
  git -C "$repo_root" merge-base --is-ancestor "$mapped" HEAD
done

git -C "$repo_root" fsck --full
echo "US-090 filtered scope verification passed"
