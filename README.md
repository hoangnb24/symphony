# Symphony

Symphony is a local orchestrator for running Harness stories. It prepares an
isolated workspace for an agent, passes the story contract explicitly, records
run results, and keeps durable Harness updates reviewable.

This repository is the standalone Symphony product. Its imported source layout
is intentionally preserved for the first standalone release:

```text
crates/harness-symphony/          Rust application
crates/harness-symphony/web-ui/   React, Playwright, and Electron UI
```

The Cargo workspace has exactly that one member and does not depend on a
checkout of `repository-harness`.

## Provisional Harness installation

The generic Harness template is installed with merge semantics so contributors
can inspect its policies and schemas. The downloaded `scripts/bin/harness-cli`
is the pre-protocol `harness-cli-v0.1.11` binary and is deliberately
provisional: do not initialize or mutate a target Harness database yet. US-093
must perform a checksum-verified forced upgrade to the exact protocol-v1
release from US-092 before creating the target durable planning state.

## Prerequisites

- Rust `1.92.0`, pinned by `rust-toolchain.toml`, with rustfmt and Clippy.
- Node.js `24.9.0`, pinned by `.node-version`.
- npm, supplied with the pinned Node.js distribution.

These conservative pins match the toolchains used to create and validate the
standalone workspace. Rustup reads the Rust pin automatically. With a Node
version manager that supports `.node-version`, select the Node pin before
installing packages (for example, `fnm use`). Do not use an existing
`node_modules` directory as proof of a clean checkout: CI and the commands
below use `npm ci` against the committed lockfile.

## Build and run

From the repository root:

```bash
cargo build --locked -p harness-symphony
cargo run --locked -p harness-symphony -- doctor
cargo run --locked -p harness-symphony -- work list
```

The product documentation starts at
[`docs/SYMPHONY_QUICKSTART.md`](docs/SYMPHONY_QUICKSTART.md) and
[`docs/SYMPHONY_SCOPE.md`](docs/SYMPHONY_SCOPE.md).

## Web and desktop development

Install exactly the dependency graph in the Web UI lockfile, then use its npm
scripts without changing directories:

```bash
npm --prefix crates/harness-symphony/web-ui ci
npm --prefix crates/harness-symphony/web-ui run build
npm --prefix crates/harness-symphony/web-ui run dev
```

The desktop smoke test builds both the Web UI and the Rust backend and verifies
their repository-relative asset discovery:

```bash
npm --prefix crates/harness-symphony/web-ui run desktop:smoke
```

## Contributor verification

Run the same gates as CI from the repository root:

```bash
cargo metadata --locked --no-deps --format-version 1
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked

npm --prefix crates/harness-symphony/web-ui ci
npm --prefix crates/harness-symphony/web-ui exec -- playwright install --with-deps chromium
npm --prefix crates/harness-symphony/web-ui run build
npm --prefix crates/harness-symphony/web-ui run e2e
npm --prefix crates/harness-symphony/web-ui run desktop:smoke
```

`playwright install --with-deps chromium` installs Linux system packages and
therefore may require elevated privileges. On macOS or Windows, install the
Chromium browser with `playwright install chromium` instead; CI executes the
full `--with-deps` form on Linux and runs the build and desktop path smoke on
all three operating systems.

## License

Symphony is available under the [MIT License](LICENSE).
