import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const [command, ...args] = process.argv.slice(2);
const sha = (data) => crypto.createHash("sha256").update(data).digest("hex");
const writeJson = (file, value) => fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`);

function files(root) {
  const result = [];
  function walk(directory) {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
      const absolute = path.join(directory, entry.name);
      if (entry.isDirectory()) walk(absolute);
      else if (entry.isFile()) result.push(path.relative(root, absolute).split(path.sep).join("/"));
      else throw new Error(`unsupported staged entry: ${absolute}`);
    }
  }
  walk(root);
  return result;
}

function treeHash(root) {
  const hash = crypto.createHash("sha256");
  for (const relative of files(root)) {
    const data = fs.readFileSync(path.join(root, relative));
    hash.update(relative); hash.update("\0"); hash.update(sha(data)); hash.update("\n");
  }
  return hash.digest("hex");
}

if (command === "tree-hash") {
  process.stdout.write(`${treeHash(args[0])}\n`);
} else if (command === "generate") {
  const [stage, output, version, sourceSha, target, binaryPath, epoch, rustVersion, nodeVersion, dirtyValue = "false"] = args;
  const webRoot = "share/harness-symphony/web-ui";
  const metadata = {
    metadata_version: 1, product: "harness-symphony", symphony_version: version,
    source_sha: sourceSha, source_dirty: dirtyValue === "true", source_date_epoch: Number(epoch), target_triple: target,
    binary_path: binaryPath, web_asset_root: webRoot, web_asset_sha256: treeHash(path.join(stage, webRoot)),
    supported_harness: { protocol_version: 1, schema_minimum: 1, schema_maximum: 13, current_schema_minimum: 12, current_schema_maximum: 13 },
    toolchain: { rust: rustVersion, node: nodeVersion }
  };
  writeJson(output, metadata);
} else if (command === "sbom") {
  const [stage, output, version, sourceSha] = args;
  writeJson(output, {
    spdxVersion: "SPDX-2.3", dataLicense: "CC0-1.0", SPDXID: "SPDXRef-DOCUMENT",
    name: `harness-symphony-${version}`, documentNamespace: `https://github.com/hoangnb24/symphony/sbom/${sourceSha}`,
    creationInfo: { creators: ["Tool: scripts/release-metadata.mjs"], created: new Date(0).toISOString() },
    packages: [{ SPDXID: "SPDXRef-Package-Symphony", name: "harness-symphony", versionInfo: version, downloadLocation: "NOASSERTION", filesAnalyzed: true }],
    files: files(stage).filter((file) => !file.endsWith("sbom.spdx.json")).map((file, index) => ({ SPDXID: `SPDXRef-File-${index + 1}`, fileName: file, checksums: [{ algorithm: "SHA256", checksumValue: sha(fs.readFileSync(path.join(stage, file))) }] }))
  });
} else if (command === "manifest") {
  const [output, metadataFile, archive, archiveSha, format, metadataSha, provenanceSha, sbomSha] = args;
  const metadata = JSON.parse(fs.readFileSync(metadataFile, "utf8"));
  writeJson(output, {
    manifest_version: 1, product: metadata.product, symphony_version: metadata.symphony_version,
    source_sha: metadata.source_sha, source_dirty: metadata.source_dirty, supported_harness: metadata.supported_harness,
    artifacts: [{ target_triple: metadata.target_triple, archive_name: path.basename(archive), archive_format: format,
      binary_path: metadata.binary_path, web_asset_root: metadata.web_asset_root,
      web_asset_sha256: metadata.web_asset_sha256, archive_sha256: archiveSha,
      metadata_sha256: metadataSha, provenance_sha256: provenanceSha, sbom_sha256: sbomSha }]
  });
} else if (command === "merge-manifests") {
  const [output, ...inputs] = args;
  if (inputs.length === 0) throw new Error("merge-manifests requires at least one input");
  const manifests = inputs.map((file) => JSON.parse(fs.readFileSync(file, "utf8")));
  const first = manifests[0];
  const identityOf = (manifest) => JSON.stringify({ manifest_version: manifest.manifest_version, product: manifest.product, symphony_version: manifest.symphony_version, source_sha: manifest.source_sha, source_dirty: manifest.source_dirty, supported_harness: manifest.supported_harness });
  for (const manifest of manifests) {
    if (identityOf(manifest) !== identityOf(first)) throw new Error("native release manifests have inconsistent top-level identity");
  }
  const artifacts = manifests.flatMap((manifest) => manifest.artifacts).sort((a, b) => a.target_triple.localeCompare(b.target_triple));
  if (new Set(artifacts.map((item) => item.target_triple)).size !== artifacts.length) throw new Error("duplicate release target triple");
  if (new Set(artifacts.map((item) => item.archive_name)).size !== artifacts.length) throw new Error("duplicate release archive name");
  const expectedTargets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "x86_64-apple-darwin", "x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu"];
  if (JSON.stringify(artifacts.map((item) => item.target_triple)) !== JSON.stringify(expectedTargets)) throw new Error("aggregate release manifest must contain exactly the five supported target triples");
  if (first.source_dirty !== false) throw new Error("aggregate release manifest cannot represent dirty source inputs");
  writeJson(output, { manifest_version: first.manifest_version, product: first.product, symphony_version: first.symphony_version, source_sha: first.source_sha, source_dirty: first.source_dirty, supported_harness: first.supported_harness, artifacts });
} else {
  throw new Error("usage: release-metadata.mjs tree-hash|generate|sbom|manifest ...");
}
