# Symphony

Symphony is a local orchestrator for running Harness stories. It discovers
runnable work through the public Harness CLI protocol, prepares an isolated
workspace, gives an agent an explicit run contract, validates its result, and
keeps product changes and durable Harness changesets reviewable.

Symphony is a standalone product. It does not require a checkout of Harness,
link Harness source, inspect Harness database tables, or copy a live SQLite
database. The typed boundary is documented in
[`docs/contracts/harness-runtime-v1.md`](docs/contracts/harness-runtime-v1.md).

## Operator workflow

An operator runs a built Symphony executable against the repository that owns
the stories. The repository can be different from the current directory.

Packaged Symphony release assets are planned in US-096 and are not published
yet. Today, obtain a locally built executable from a Symphony contributor and
set its path explicitly:

```bash
SYMPHONY=/absolute/path/to/harness-symphony
REPO=/absolute/path/to/your-harness-repository

"$SYMPHONY" --repo-root "$REPO" doctor
"$SYMPHONY" --repo-root "$REPO" work list
"$SYMPHONY" --repo-root "$REPO" run <story-id> --prepare-only
```

```powershell
$Symphony = "C:\absolute\path\to\harness-symphony.exe"
$Repo = "C:\absolute\path\to\your-harness-repository"

& $Symphony --repo-root $Repo doctor
& $Symphony --repo-root $Repo work list
& $Symphony --repo-root $Repo run <story-id> --prepare-only
```

The target repository must have a compatible Harness CLI and Harness database.
`doctor` reports the resolved CLI and any corrective action. See the
[`Quick Start`](docs/SYMPHONY_QUICKSTART.md) for the complete run, review, PR,
and sync loop. A configuration template is available at
[`examples/symphony.yml`](examples/symphony.yml); copy it to the target
repository as `.harness/symphony.yml` only when defaults are insufficient.

## Contributor workflow

The source workspace currently contains one Rust application and its Web UI:

```text
crates/harness-symphony/          Rust application
crates/harness-symphony/web-ui/   React, Playwright, and Electron UI
```

Prerequisites are Rust `1.92.0` (pinned by `rust-toolchain.toml`) and Node.js
`24.9.0` (pinned by `.node-version`). Build and exercise the source checkout
from its repository root:

```bash
cargo build --locked -p harness-symphony
cargo run --locked -p harness-symphony -- --repo-root /path/to/target doctor
cargo run --locked -p harness-symphony -- --repo-root /path/to/target work list

npm --prefix crates/harness-symphony/web-ui ci
npm --prefix crates/harness-symphony/web-ui run build
npm --prefix crates/harness-symphony/web-ui run dev
```

Run the CI-equivalent checks before submitting a source change:

```bash
cargo metadata --locked --no-deps --format-version 1
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked

npm --prefix crates/harness-symphony/web-ui ci
npm --prefix crates/harness-symphony/web-ui exec -- playwright install chromium
npm --prefix crates/harness-symphony/web-ui run build
npm --prefix crates/harness-symphony/web-ui run e2e

FIXTURE=$(mktemp -d)
tests/compatibility/bootstrap-harness-fixture.sh --upgrade-cli \
  --story US-DESKTOP-SMOKE "$FIXTURE"
npm --prefix crates/harness-symphony/web-ui run desktop:smoke -- \
  --repo-root "$FIXTURE"
rm -rf "$FIXTURE"
```

On Linux CI, Playwright may use `playwright install --with-deps chromium`; that
can require elevated privileges because it installs system packages.

## Product contract

- [`docs/SYMPHONY_QUICKSTART.md`](docs/SYMPHONY_QUICKSTART.md) — operator loop.
- [`docs/SYMPHONY_SCOPE.md`](docs/SYMPHONY_SCOPE.md) — implemented contract and
  future boundary.
- [`docs/contracts/harness-runtime-v1.md`](docs/contracts/harness-runtime-v1.md)
  — pinned external protocol.

Symphony is available under the [MIT License](LICENSE).
