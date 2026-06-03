#!/usr/bin/env node

const { chmodSync, copyFileSync, existsSync, mkdirSync } = require("node:fs");
const { join, resolve } = require("node:path");

const rootDir = resolve(__dirname, "..");
const npmDir = join(rootDir, "npm");
const binariesDir = join(npmDir, "binaries");

const platforms = [
  { os: "darwin", arch: "arm64", binaryName: "niteo" },
  { os: "darwin", arch: "x64", binaryName: "niteo" },
  { os: "linux", arch: "x64", binaryName: "niteo" },
  { os: "linux", arch: "arm64", binaryName: "niteo" },
  { os: "win32", arch: "x64", binaryName: "niteo.exe" },
];

console.log("Assembling root npm package binaries...");

for (const { os, arch, binaryName } of platforms) {
  const platformKey = `${os}-${arch}`;
  const sourceBinary = join(binariesDir, `${os}-${arch}`, binaryName);
  const destDir = join(rootDir, "bin", platformKey);
  const destBinary = join(destDir, binaryName);

  console.log(`\nProcessing ${platformKey}...`);

  if (!existsSync(sourceBinary)) {
    console.error(`  Error: Binary not found at ${sourceBinary}`);
    process.exit(1);
  }

  mkdirSync(destDir, { recursive: true });
  copyFileSync(sourceBinary, destBinary);
  if (os !== "win32") {
    chmodSync(destBinary, 0o755);
  }
  console.log(`  Copied binary to ${destBinary}`);
}

console.log("\nRoot npm package binaries assembled successfully.");
