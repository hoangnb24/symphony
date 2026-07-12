const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const electronPath = require("electron");
const {
  developmentBackendBinary,
  findRepoRoot,
  repoRootArgument,
  repoRootFromElectronDir,
  requestText,
  startBackend,
  waitForHttp
} = require("./backend.cjs");

const sourceRoot = repoRootFromElectronDir();
const webUiRoot = path.resolve(__dirname, "..");

function runChecked(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? sourceRoot,
    stdio: "inherit",
    shell: process.platform === "win32"
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

function checkSyntax(file) {
  runChecked(process.execPath, ["--check", file], { cwd: webUiRoot });
}

function assertOldCratePathIsRejected() {
  const decoy = fs.mkdtempSync(path.join(os.tmpdir(), "symphony-old-crate-decoy-"));
  try {
    fs.mkdirSync(path.join(decoy, "crates", "harness-symphony"), { recursive: true });
    fs.writeFileSync(path.join(decoy, "crates", "harness-symphony", "Cargo.toml"), "[package]\n");
    let rejected = false;
    try {
      findRepoRoot({ repoRoot: decoy });
    } catch (error) {
      rejected = /does not contain Symphony config/.test(error.message);
    }
    if (!rejected) {
      throw new Error("A directory containing only the retired Symphony crate path was accepted");
    }
  } finally {
    fs.rmSync(decoy, { recursive: true, force: true });
  }
}

function assertIncompatibleHarnessCliIsRejected() {
  const decoy = fs.mkdtempSync(path.join(os.tmpdir(), "symphony-incompatible-cli-"));
  try {
    fs.mkdirSync(path.join(decoy, ".harness"), { recursive: true });
    fs.writeFileSync(
      path.join(decoy, ".harness", "symphony.yml"),
      `version: 1\nrepo:\n  harness_cli: ${JSON.stringify(process.execPath)}\n`
    );
    fs.writeFileSync(path.join(decoy, "harness.db"), "not a compatible Harness database");
    let rejected = false;
    try {
      findRepoRoot({ repoRoot: decoy });
    } catch (error) {
      rejected = /compatible Harness CLI\/database contract/.test(error.message);
    }
    if (!rejected) {
      throw new Error("A present but protocol-incompatible Harness CLI was accepted");
    }
  } finally {
    fs.rmSync(decoy, { recursive: true, force: true });
  }
}

async function main() {
  checkSyntax(path.join(webUiRoot, "electron", "backend.cjs"));
  checkSyntax(path.join(webUiRoot, "electron", "main.cjs"));
  checkSyntax(path.join(webUiRoot, "electron", "dev.cjs"));
  if (!fs.existsSync(electronPath)) {
    throw new Error(`Electron executable is missing: ${electronPath}`);
  }

  const selected = repoRootArgument();
  if (!selected) {
    throw new Error("Desktop smoke requires --repo-root <external Harness fixture>");
  }
  const fixtureRoot = findRepoRoot({ repoRoot: selected });
  assertOldCratePathIsRejected();
  assertIncompatibleHarnessCliIsRejected();

  runChecked("cargo", ["build", "-p", "harness-symphony"]);
  const backend = startBackend({
    repoRoot: fixtureRoot,
    binary: developmentBackendBinary(sourceRoot),
    assetDir: path.join(webUiRoot, "dist"),
    port: 0
  });

  try {
    const baseUrl = await backend.urlPromise;
    const health = await waitForHttp(`${baseUrl}/health`, { timeoutMs: 30000 });
    if (health.statusCode !== 200 || JSON.parse(health.body).ok !== true) {
      throw new Error("Desktop backend health response was invalid");
    }
    const root = await waitForHttp(baseUrl, { timeoutMs: 30000 });
    if (root.statusCode !== 200 || !root.body.includes("<div id=\"root\"></div>")) {
      throw new Error("Desktop backend did not serve the built React index");
    }
    const board = await requestText(`${baseUrl}/api/board`);
    if (board.statusCode !== 200 || !Array.isArray(JSON.parse(board.body).items)) {
      throw new Error(`/api/board returned invalid response (HTTP ${board.statusCode})`);
    }
    console.log(`Desktop external-fixture smoke passed at ${baseUrl}`);
  } finally {
    await backend.stop();
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
