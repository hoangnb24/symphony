#!/usr/bin/env bash
set -euo pipefail

bundle=${1:?usage: smoke-standalone-web.sh BUNDLE FIXTURE}
fixture=${2:?usage: smoke-standalone-web.sh BUNDLE FIXTURE}
binary_name=harness-symphony
[[ "${OS:-}" == Windows_NT ]] && binary_name=harness-symphony.exe
binary="$bundle/$binary_name"
log=$(mktemp)
body=$(mktemp)
pid=
third_directory=

descendants_of() {
  local parent=$1 child
  command -v pgrep >/dev/null 2>&1 || return 0
  while IFS= read -r child; do
    [[ -n "$child" ]] || continue
    descendants_of "$child"
    printf '%s\n' "$child"
  done < <(pgrep -P "$parent" 2>/dev/null || true)
}

stop_process_tree() {
  local root_pid=$1 descendants child
  if [[ "${OS:-}" == Windows_NT ]]; then
    taskkill //PID "$root_pid" //T //F >/dev/null 2>&1 || true
    return
  fi
  descendants=$(descendants_of "$root_pid")
  while IFS= read -r child; do
    [[ -n "$child" ]] && kill "$child" 2>/dev/null || true
  done <<<"$descendants"
  kill "$root_pid" 2>/dev/null || true
  for _ in {1..50}; do
    local alive=0
    kill -0 "$root_pid" 2>/dev/null && alive=1
    while IFS= read -r child; do
      [[ -n "$child" ]] && kill -0 "$child" 2>/dev/null && alive=1
    done <<<"$descendants"
    [[ "$alive" == 0 ]] && return
    sleep 0.1
  done
  while IFS= read -r child; do
    [[ -n "$child" ]] && kill -9 "$child" 2>/dev/null || true
  done <<<"$descendants"
  kill -9 "$root_pid" 2>/dev/null || true
  return 1
}

cleanup() {
  local status=$?
  if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
    if ! stop_process_tree "$pid"; then
      status=1
      echo "standalone Web process tree did not terminate cleanly" >&2
    fi
    wait "$pid" 2>/dev/null || true
  fi
  rm -f "$log" "$body"
  [[ -z "$third_directory" ]] || rm -rf "$third_directory"
  exit "$status"
}
trap cleanup EXIT INT TERM

[[ -x "$binary" ]] || { echo "missing standalone binary: $binary" >&2; exit 1; }
[[ -f "$bundle/web-ui-dist/index.html" ]] || { echo "missing standalone Web assets" >&2; exit 1; }

third_directory=$(mktemp -d)
(cd "$third_directory" && "$binary" --repo-root "$fixture" web --host 127.0.0.1 --port 0) >"$log" 2>&1 &
pid=$!

base_url=
for _ in {1..300}; do
  base_url=$(sed -n 's/.*\(http:\/\/127\.0\.0\.1:[0-9][0-9]*\).*/\1/p' "$log" | tail -1)
  [[ -n "$base_url" ]] && break
  kill -0 "$pid" 2>/dev/null || { cat "$log" >&2; exit 1; }
  sleep 0.1
done
[[ -n "$base_url" ]] || { echo "timed out waiting for standalone Web readiness" >&2; cat "$log" >&2; exit 1; }

curl -fsS "$base_url/health" >"$body"
grep -Eq '"ok"[[:space:]]*:[[:space:]]*true' "$body"
curl -fsS "$base_url/api/board" >"$body"
grep -Eq '"items"[[:space:]]*:' "$body"
curl -fsS "$base_url/" >"$body"
grep -Fq '<div id="root"></div>' "$body"

assets=$(sed -n "s#.*\\(/assets/[^\\\"' ]*\\).*#\\1#p" "$body" | LC_ALL=C sort -u)
[[ -n "$assets" ]] || { echo "built index did not reference any assets" >&2; exit 1; }
while IFS= read -r asset; do
  headers=$(mktemp)
  curl -fsS -D "$headers" "$base_url$asset" >"$body"
  grep -Eiq '^content-type: (application/javascript|text/css)' "$headers"
  rm -f "$headers"
  test -s "$body"
done <<<"$assets"

echo "Standalone Web smoke passed at $base_url"
