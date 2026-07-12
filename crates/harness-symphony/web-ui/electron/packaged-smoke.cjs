const fs = require("node:fs");
const path = require("node:path");
const {
  findRepoRoot,
  packagedBackendBinary,
  repoRootArgument,
  requestText,
  startBackend,
  waitForHttp
} = require("./backend.cjs");

const webUiRoot = path.resolve(__dirname, "..");

function findResourceRoots(root) {
  const matches = [];
  function walk(directory) {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const absolute = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        walk(absolute);
      } else if (
        entry.name === "resource-manifest.json" &&
        path.basename(path.dirname(absolute)) === "harness-symphony" &&
        path.basename(path.dirname(path.dirname(absolute))) === "share"
      ) {
        matches.push(path.dirname(path.dirname(path.dirname(absolute))));
      }
    }
  }
  walk(root);
  return [...new Set(matches.map((value) => path.resolve(value)))];
}

async function main() {
  const selected = repoRootArgument();
  if (!selected) {
    throw new Error("Packaged desktop smoke requires --repo-root <external Harness fixture>");
  }
  const fixtureRoot = findRepoRoot({ repoRoot: selected });
  const packageRoot = path.resolve(process.env.SYMPHONY_DESKTOP_PACKAGE_DIR || path.join(webUiRoot, "desktop-dist"));
  if (!fs.existsSync(packageRoot)) {
    throw new Error(`Packaged Electron output is missing: ${packageRoot}`);
  }
  const resources = findResourceRoots(packageRoot);
  if (resources.length !== 1) {
    throw new Error(`Expected one packaged Electron resources directory, found ${resources.length}`);
  }
  const resourcesPath = resources[0];
  const manifestPath = path.join(resourcesPath, "share", "harness-symphony", "resource-manifest.json");
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  const expectedBinary = process.platform === "win32"
    ? "bin/harness-symphony.exe"
    : "bin/harness-symphony";
  if (
    manifest.format_version !== 1 ||
    manifest.binary_path !== expectedBinary ||
    manifest.web_asset_root !== "share/harness-symphony/web-ui"
  ) {
    throw new Error(`Packaged resource manifest does not match this platform: ${manifestPath}`);
  }
  const binary = packagedBackendBinary(resourcesPath);
  if (!fs.existsSync(binary)) {
    throw new Error(`Packaged backend lookup failed: ${binary}`);
  }
  if (!fs.existsSync(path.join(resourcesPath, manifest.web_asset_root, "index.html"))) {
    throw new Error("Packaged Web resource root is incomplete");
  }
  const appPayload = process.platform === "darwin"
    ? path.join(resourcesPath, "app.asar")
    : path.join(resourcesPath, "app.asar");
  if (!fs.existsSync(appPayload)) {
    throw new Error(`Packaged Electron application payload is missing: ${appPayload}`);
  }

  const backend = startBackend({
    repoRoot: fixtureRoot,
    binary,
    port: 0,
    clearAssetOverride: true
  });
  try {
    const baseUrl = await backend.urlPromise;
    const health = await waitForHttp(`${baseUrl}/health`, { timeoutMs: 30000 });
    const root = await waitForHttp(baseUrl, { timeoutMs: 30000 });
    const board = await requestText(`${baseUrl}/api/board`);
    if (health.statusCode !== 200 || JSON.parse(health.body).ok !== true) {
      throw new Error("Packaged backend health response was invalid");
    }
    if (root.statusCode !== 200 || !root.body.includes("<div id=\"root\"></div>")) {
      throw new Error("Packaged backend did not serve its manifest-selected Web UI");
    }
    if (board.statusCode !== 200 || !Array.isArray(JSON.parse(board.body).items)) {
      throw new Error(`Packaged /api/board response was invalid (HTTP ${board.statusCode})`);
    }
    console.log(`Packaged Electron resources smoke passed at ${baseUrl}`);
  } finally {
    await backend.stop();
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
