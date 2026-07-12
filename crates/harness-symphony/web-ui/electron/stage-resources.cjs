const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");

const webUiRoot = path.resolve(__dirname, "..");
const repoRoot = path.resolve(webUiRoot, "../../..");
const binaryName = process.platform === "win32" ? "harness-symphony.exe" : "harness-symphony";
const binarySource = path.join(repoRoot, "target", "release", binaryName);
const stagingRoot = path.join(webUiRoot, "desktop-resources");
const webRoot = path.join(stagingRoot, "share", "harness-symphony", "web-ui");

function filesBelow(root, relative = "") {
  return fs.readdirSync(path.join(root, relative), { withFileTypes: true }).flatMap((entry) => {
    const child = path.join(relative, entry.name);
    return entry.isDirectory() ? filesBelow(root, child) : [child];
  });
}

function webTreeHash(root) {
  const hash = crypto.createHash("sha256");
  for (const relative of filesBelow(root).map((value) => value.split(path.sep).join("/")).sort()) {
    const fileHash = crypto.createHash("sha256").update(fs.readFileSync(path.join(root, relative))).digest("hex");
    hash.update(`${relative}\0${fileHash}\n`);
  }
  return hash.digest("hex");
}

if (!fs.existsSync(binarySource)) {
  throw new Error(`Release backend is missing: ${binarySource}`);
}
if (!fs.existsSync(path.join(webUiRoot, "dist", "index.html"))) {
  throw new Error("Built Web UI is missing; run npm run build first");
}

fs.rmSync(stagingRoot, { recursive: true, force: true });
fs.mkdirSync(path.join(stagingRoot, "bin"), { recursive: true });
fs.cpSync(path.join(webUiRoot, "dist"), webRoot, { recursive: true });
fs.copyFileSync(binarySource, path.join(stagingRoot, "bin", binaryName));
fs.chmodSync(path.join(stagingRoot, "bin", binaryName), 0o755);

const resourceRoot = path.join(stagingRoot, "share", "harness-symphony");
fs.writeFileSync(
  path.join(resourceRoot, "resource-manifest.json"),
  `${JSON.stringify(
    {
      format_version: 1,
      binary_path: `bin/${binaryName}`,
      web_asset_root: "share/harness-symphony/web-ui",
      web_asset_sha256: webTreeHash(webRoot)
    },
    null,
    2
  )}\n`
);
console.log(`Staged Electron resources at ${stagingRoot}`);
