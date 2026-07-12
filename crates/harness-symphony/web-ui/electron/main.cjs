const path = require("node:path");
const { app, BrowserWindow, dialog, shell } = require("electron");
const {
  developmentBackendBinary,
  findRepoRoot,
  packagedBackendBinary,
  repoRootArgument,
  repoRootFromElectronDir,
  startBackend,
  waitForHttp
} = require("./backend.cjs");

const isDev = process.argv.includes("--dev") || process.env.SYMPHONY_DESKTOP_DEV === "1";
let backend = null;

function desktopPaths() {
  const repoRoot = findRepoRoot({
    repoRoot: repoRootArgument(),
    electronDir: __dirname,
    resourcesPath: process.resourcesPath,
    cwd: process.cwd()
  });
  if (app.isPackaged && !isDev) {
    return {
      repoRoot,
      binary: packagedBackendBinary(),
      // Let the packaged backend validate the shared resource manifest and
      // resolve ../share/harness-symphony/web-ui from its bin/ location.
      assetDir: undefined,
      clearAssetOverride: true,
      port: 0,
      loadVite: false
    };
  }

  return {
    repoRoot,
    binary:
      process.env.SYMPHONY_BACKEND_BINARY || developmentBackendBinary(repoRootFromElectronDir()),
    assetDir: path.resolve(__dirname, "..", "dist"),
    port: Number(process.env.SYMPHONY_BACKEND_PORT || "4317"),
    loadVite: true
  };
}

function createWindow(loadUrl) {
  const window = new BrowserWindow({
    width: 1440,
    height: 940,
    minWidth: 1180,
    minHeight: 760,
    title: "Harness Symphony",
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true
    }
  });

  window.webContents.setWindowOpenHandler(({ url }) => {
    shell.openExternal(url);
    return { action: "deny" };
  });
  window.loadURL(loadUrl);
  return window;
}

async function start() {
  const paths = desktopPaths();
  backend = startBackend(paths);
  const backendUrl = await backend.urlPromise;
  await waitForHttp(`${backendUrl}/health`, { timeoutMs: 30000 });
  const loadUrl = paths.loadVite
    ? process.env.SYMPHONY_DESKTOP_URL || "http://127.0.0.1:5177"
    : backendUrl;
  createWindow(loadUrl);
}

app.whenReady().then(() => {
  start().catch((error) => {
    dialog.showErrorBox("Harness Symphony failed to start", error.message);
    app.quit();
  });
});

app.on("window-all-closed", () => {
  app.quit();
});

app.on("before-quit", () => {
  if (backend) {
    backend.stop();
  }
});
