const fs = require("node:fs");
const http = require("node:http");
const path = require("node:path");
const { spawn, spawnSync } = require("node:child_process");

const REQUIRED_CAPABILITIES = [
  "changesets.apply.v1",
  "changesets.status-sha.v1",
  "isolated-db-snapshot.v1",
  "isolated-db.v1",
  "semantic-operation-log.v1",
  "stories.read.v1",
  "stories.write.v1",
  "story-dependencies.read-write.v1",
  "story-hierarchy.read-write.v1",
  "work-graph.read.v1"
];

function repoRootFromElectronDir() {
  return path.resolve(__dirname, "../../../..");
}

function looksLikeRepoRoot(candidate) {
  if (!candidate) {
    return false;
  }
  const configPath = path.join(candidate, ".harness", "symphony.yml");
  const hasSymphonyConfig = fs.existsSync(configPath);
  const hasHarnessDatabase = fs.existsSync(path.join(candidate, "harness.db"));
  let configText = "";
  let configuredCli;
  if (hasSymphonyConfig) {
    configText = fs.readFileSync(configPath, "utf8");
    const match = configText.match(/^\s*harness_cli:\s*["']?([^\n"']+)["']?\s*$/m);
    configuredCli = match && match[1].trim();
  }
  const cliValue = configuredCli || process.env.HARNESS_CLI_PATH;
  const harnessCli = cliValue
    ? path.resolve(candidate, cliValue)
    : path.join(
        candidate,
        "scripts",
        "bin",
        process.platform === "win32" ? "harness-cli.exe" : "harness-cli"
      );
  const configVersion = configText.match(/^\s*version:\s*(\d+)\s*$/m);
  return (
    (!hasSymphonyConfig || (configVersion && configVersion[1] === "1")) &&
    hasHarnessDatabase &&
    fs.existsSync(harnessCli) &&
    compatibleHarnessContract(candidate, harnessCli)
  );
}

function compatibleHarnessContract(repoRoot, harnessCli) {
  const result = spawnSync(harnessCli, ["query", "contract", "--json"], {
    cwd: repoRoot,
    env: {
      ...process.env,
      HARNESS_REPO_ROOT: repoRoot,
      HARNESS_DB_PATH: path.join(repoRoot, "harness.db")
    },
    encoding: "utf8",
    timeout: 10000,
    windowsHide: true
  });
  if (result.error || result.status !== 0) {
    return false;
  }
  let envelope;
  try {
    envelope = JSON.parse(result.stdout);
  } catch {
    return false;
  }
  const contract = envelope && envelope.operation === "query.contract" && envelope.result;
  if (
    !contract ||
    envelope.protocol_version !== 1 ||
    contract.protocol_version !== 1 ||
    contract.cli_version !== "0.1.14" ||
    contract.database_state !== "current" ||
    ![12, 13].includes(contract.database_schema_version) ||
    contract.schema_minimum !== 1 ||
    contract.schema_maximum !== 13 ||
    !Array.isArray(contract.required_environment_variables) ||
    !contract.required_environment_variables.includes("HARNESS_DB_PATH") ||
    !Array.isArray(contract.capabilities)
  ) {
    return false;
  }
  return REQUIRED_CAPABILITIES.every((capability) => contract.capabilities.includes(capability));
}

function repoRootArgument(argv = process.argv.slice(1)) {
  const index = argv.indexOf("--repo-root");
  if (index === -1) {
    return undefined;
  }
  const value = argv[index + 1];
  if (!value || value.startsWith("--")) {
    throw new Error("--repo-root requires a directory path");
  }
  return value;
}

function ancestors(startPath) {
  const result = [];
  let current = path.resolve(startPath);
  for (;;) {
    result.push(current);
    const parent = path.dirname(current);
    if (parent === current) {
      return result;
    }
    current = parent;
  }
}

function findRepoRoot(options = {}) {
  const explicit = options.repoRoot || options.envRepoRoot || process.env.SYMPHONY_REPO_ROOT;
  if (explicit) {
    if (!looksLikeRepoRoot(explicit)) {
      throw new Error(
        `Selected repository does not provide a compatible Harness CLI/database contract: ${explicit}`
      );
    }
    return path.resolve(explicit);
  }

  const starts = [
    options.cwd || process.cwd(),
    options.electronDir || __dirname,
    options.resourcesPath || process.resourcesPath
  ].filter(Boolean);

  for (const start of starts) {
    for (const candidate of ancestors(start)) {
      if (looksLikeRepoRoot(candidate)) {
        return candidate;
      }
    }
  }

  throw new Error(
    "Could not find a Harness project configured for Symphony. Pass --repo-root or set SYMPHONY_REPO_ROOT."
  );
}

function platformBinaryName() {
  return process.platform === "win32" ? "harness-symphony.exe" : "harness-symphony";
}

function developmentBackendBinary(repoRoot) {
  return path.join(repoRoot, "target", "debug", platformBinaryName());
}

function packagedBackendBinary(resourcesPath = process.resourcesPath) {
  return path.join(resourcesPath, "bin", platformBinaryName());
}

function assertExecutable(binaryPath) {
  if (!fs.existsSync(binaryPath)) {
    throw new Error(`Harness Symphony backend binary is missing: ${binaryPath}`);
  }
}

function requestText(url) {
  return new Promise((resolve, reject) => {
    const request = http.get(url, (response) => {
      let body = "";
      response.setEncoding("utf8");
      response.on("data", (chunk) => {
        body += chunk;
      });
      response.on("end", () => {
        resolve({
          statusCode: response.statusCode ?? 0,
          body
        });
      });
    });
    request.on("error", reject);
    request.setTimeout(1500, () => {
      request.destroy(new Error(`Timed out waiting for ${url}`));
    });
  });
}

async function waitForHttp(url, options = {}) {
  const timeoutMs = options.timeoutMs ?? 30000;
  const startedAt = Date.now();
  let lastError;

  while (Date.now() - startedAt < timeoutMs) {
    try {
      const response = await requestText(url);
      if (response.statusCode >= 200 && response.statusCode < 500) {
        return response;
      }
      lastError = new Error(`${url} returned HTTP ${response.statusCode}`);
    } catch (error) {
      lastError = error;
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }

  throw lastError ?? new Error(`Timed out waiting for ${url}`);
}

function startBackend(options) {
  const repoRoot = options.repoRoot;
  const binary = options.binary;
  const assetDir = options.assetDir;
  const host = options.host ?? "127.0.0.1";
  const port = options.port ?? 0;
  assertExecutable(binary);
  const environment = { ...process.env };
  if (options.clearAssetOverride) {
    delete environment.HARNESS_SYMPHONY_WEB_DIST_DIR;
  } else if (assetDir) {
    environment.HARNESS_SYMPHONY_WEB_DIST_DIR = assetDir;
  }

  const child = spawn(
    binary,
    ["--repo-root", repoRoot, "web", "--host", host, "--port", String(port)],
    {
      cwd: repoRoot,
      env: environment,
      stdio: ["ignore", "pipe", "pipe"]
    }
  );

  let settled = false;
  let stdout = "";
  let stderr = "";

  const urlPromise = new Promise((resolve, reject) => {
    const fail = (error) => {
      if (!settled) {
        settled = true;
        reject(error);
      }
    };

    const parseUrl = (chunk) => {
      const text = chunk.toString();
      stdout += text;
      const match = text.match(/http:\/\/127\.0\.0\.1:\d+/);
      if (match && !settled) {
        settled = true;
        resolve(match[0]);
      }
    };

    child.stdout.on("data", parseUrl);
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });
    child.on("error", fail);
    child.on("exit", (code, signal) => {
      if (!settled) {
        fail(
          new Error(
            `Harness Symphony backend exited before startup (code ${code}, signal ${signal}). ${stderr || stdout}`
          )
        );
      }
    });
  });

  return {
    process: child,
    urlPromise,
    async healthUrl() {
      const url = await urlPromise;
      return `${url}/health`;
    },
    stop() {
      if (child.exitCode !== null) {
        return Promise.resolve();
      }
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => {
          child.kill("SIGKILL");
          reject(new Error("Harness Symphony backend did not stop within 5 seconds"));
        }, 5000);
        child.once("exit", () => {
          clearTimeout(timer);
          resolve();
        });
        child.kill();
      });
    }
  };
}

module.exports = {
  assertExecutable,
  compatibleHarnessContract,
  developmentBackendBinary,
  findRepoRoot,
  looksLikeRepoRoot,
  packagedBackendBinary,
  repoRootFromElectronDir,
  repoRootArgument,
  requestText,
  startBackend,
  waitForHttp
};
