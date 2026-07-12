#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

const root = path.resolve(process.argv[2] ?? process.cwd());
const quickstart = fs.readFileSync(path.join(root, "docs/SYMPHONY_QUICKSTART.md"), "utf8");
const readme = fs.readFileSync(path.join(root, "README.md"), "utf8");

function fencedBlocks(markdown, language) {
  const blocks = [];
  const pattern = new RegExp("```" + language + "\\n([\\s\\S]*?)\\n```", "g");
  for (const match of markdown.matchAll(pattern)) blocks.push(match[1]);
  return blocks.join("\n");
}

const bash = fencedBlocks(quickstart, "bash");
const powershell = fencedBlocks(quickstart, "powershell");
const commands = [
  "doctor",
  "work list",
  "run <story-id> --prepare-only",
  "run <story-id>",
  "run <story-id> --here",
  "status",
  "runs list",
  "runs show <run_id>",
  "pr create <run_id>",
  "pr retry <run_id>",
  "sync",
];

for (const command of commands) {
  const bashLine = `"$SYMPHONY" --repo-root "$REPO" ${command}`;
  const powershellLine = `& $Symphony --repo-root $Repo ${command}`;
  if (!bash.includes(bashLine)) throw new Error(`missing Bash operator example: ${bashLine}`);
  if (!powershell.includes(powershellLine)) {
    throw new Error(`missing PowerShell operator example: ${powershellLine}`);
  }
}

for (const [name, markdown] of [["README", readme], ["Quickstart", quickstart]]) {
  const bashBlocks = fencedBlocks(markdown, "bash");
  const powershellBlocks = fencedBlocks(markdown, "powershell");
  if (!bashBlocks.includes('SYMPHONY=/absolute/path/to/harness-symphony')) {
    throw new Error(`${name} does not define the Bash artifact path`);
  }
  if (!powershellBlocks.includes('$Symphony = "C:\\absolute\\path\\to\\harness-symphony.exe"')) {
    throw new Error(`${name} does not define the PowerShell .exe artifact path`);
  }
  for (const line of bashBlocks.split("\n").filter((line) => line.startsWith('"$SYMPHONY"'))) {
    if (!line.includes('--repo-root "$REPO"')) throw new Error(`${name} Bash command omits --repo-root: ${line}`);
  }
  for (const line of powershellBlocks.split("\n").filter((line) => line.startsWith("& $Symphony"))) {
    if (!line.includes("--repo-root $Repo")) {
      throw new Error(`${name} PowerShell command omits --repo-root: ${line}`);
    }
  }
}

console.log(`checked ${commands.length} operator commands in Bash and PowerShell`);
