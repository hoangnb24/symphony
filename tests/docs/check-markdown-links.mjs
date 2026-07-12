#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const root = path.resolve(process.argv[2] ?? path.join(import.meta.dirname, "../.."));
const files = [];
function walk(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
    if ([".git", "node_modules", "target"].includes(entry.name)) continue;
    const candidate = path.join(directory, entry.name);
    if (entry.isDirectory()) walk(candidate);
    else if (entry.isFile() && entry.name.endsWith(".md")) files.push(candidate);
  }
}
walk(root);

const failures = [];
const linkPattern = /!?(?:\[[^\]]*\])\(([^)]+)\)/g;
for (const file of files) {
  const text = fs.readFileSync(file, "utf8");
  for (const match of text.matchAll(linkPattern)) {
    let target = match[1].trim();
    if (target.startsWith("<") && target.endsWith(">")) target = target.slice(1, -1);
    if (!target || target.startsWith("#") || /^[a-z][a-z0-9+.-]*:/i.test(target)) continue;
    target = target.split("#", 1)[0].split("?", 1)[0];
    try { target = decodeURIComponent(target); } catch { failures.push(`${path.relative(root, file)}: invalid URL encoding: ${target}`); continue; }
    const destination = path.resolve(path.dirname(file), target);
    if (!fs.existsSync(destination)) failures.push(`${path.relative(root, file)}: missing ${match[1]}`);
  }
}

if (failures.length) {
  console.error(failures.sort().join("\n"));
  process.exit(1);
}
console.log(`checked ${files.length} Markdown files`);
