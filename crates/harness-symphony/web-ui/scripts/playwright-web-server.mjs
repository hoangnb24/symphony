import { spawn, spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "../../../..");
const binaryName = process.platform === "win32" ? "harness-symphony.exe" : "harness-symphony";
const binary = path.join(repositoryRoot, "target", "debug", binaryName);

const build = spawnSync("cargo", ["build", "-p", "harness-symphony"], {
  cwd: repositoryRoot,
  env: process.env,
  stdio: "inherit",
  shell: false,
  windowsHide: true
});
if (build.error) throw build.error;
if (build.status !== 0) process.exit(build.status ?? 1);

const args = [];
if (process.env.PLAYWRIGHT_REPO_ROOT) {
  args.push("--repo-root", process.env.PLAYWRIGHT_REPO_ROOT);
}
args.push("web", "--host", "127.0.0.1", "--port", "43219");

const backend = spawn(binary, args, {
  cwd: repositoryRoot,
  env: process.env,
  stdio: "inherit",
  shell: false,
  windowsHide: true
});

let stopping = false;
function stop(signal) {
  if (stopping) return;
  stopping = true;
  if (!backend.killed) backend.kill(signal);
}

process.on("SIGINT", () => stop("SIGINT"));
process.on("SIGTERM", () => stop("SIGTERM"));
backend.on("error", (error) => {
  console.error(error);
  process.exitCode = 1;
});
backend.on("exit", (code, signal) => {
  if (signal && !stopping) console.error(`Playwright backend exited from signal ${signal}`);
  process.exit(code ?? (stopping ? 0 : 1));
});
