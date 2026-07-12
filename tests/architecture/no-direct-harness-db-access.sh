#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
src="$repo_root/crates/harness-symphony/src"
production=(work.rs run.rs sync.rs doctor.rs agent.rs)

fail() {
  echo "architecture violation: $*" >&2
  exit 1
}

for name in "${production[@]}"; do
  file="$src/$name"
  [[ -f "$file" ]] || fail "missing coupling-inventory file: $name"
  production_source=$(awk '/^#\[cfg\(.*test/{exit} {print}' "$file")

  if printf '%s\n' "$production_source" | rg -n 'rusqlite|Connection::open|prepare\(|query_row\(|execute(_batch)?\(' >/tmp/e11-architecture-sql.txt; then
    cat /tmp/e11-architecture-sql.txt >&2
    fail "$name still opens or queries SQLite; only state.rs may own .symphony/state.db"
  fi

  if printf '%s\n' "$production_source" | rg -n 'fs::copy\([^\n]*(harness_db|harness\.db)|copy\([^\n]*(harness_db|harness\.db)' >/tmp/e11-architecture-copy.txt; then
    cat /tmp/e11-architecture-copy.txt >&2
    fail "$name still byte-copies a Harness database"
  fi
done

rg -q 'HarnessProtocol' "$src/work.rs" || fail "work.rs does not use HarnessProtocol"
rg -q 'HarnessProtocol' "$src/run.rs" || fail "run.rs does not use HarnessProtocol"
rg -q 'HarnessProtocol' "$src/sync.rs" || fail "sync.rs does not use HarnessProtocol"
rg -q 'HarnessProtocol' "$src/doctor.rs" || fail "doctor.rs does not use HarnessProtocol"
rg -q 'HarnessProtocol|harness_cli' "$src/agent.rs" || fail "agent.rs lacks structured Harness CLI data"

rg -q 'rusqlite' "$src/state.rs" || fail "state.rs must remain the explicit product-state SQLite owner"
if rg -n 'harness_db' "$src/state.rs" >/tmp/e11-architecture-state.txt; then
  cat /tmp/e11-architecture-state.txt >&2
  fail "state.rs must not reach the Harness database"
fi

echo "US-093 architecture boundary passed: Harness DB access is protocol-only"
