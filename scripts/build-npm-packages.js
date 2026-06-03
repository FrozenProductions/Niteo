#!/usr/bin/env node

const { mkdirSync, copyFileSync, existsSync, readFileSync, writeFileSync } = require("node:fs");
const { join, resolve } = require("node:path");
const { execSync } = require("node:child_process");

const rootDir = resolve(__dirname, "..");
const npmDir = join(rootDir, "npm");
const binariesDir = join(npmDir, "binaries");

const platforms = [
  { os: "darwin", arch: "arm64", binaryName: "niteo", binEntry: "bin/niteo" },
  { os: "darwin", arch: "x64", binaryName: "niteo", binEntry: "bin/niteo" },
  { os: "linux", arch: "x64", binaryName: "niteo", binEntry: "bin/niteo" },
  { os: "linux", arch: "arm64", binaryName: "niteo", binEntry: "bin/niteo" },
  { os: "win32", arch: "x64", binaryName: "niteo.exe", binEntry: "bin/niteo.exe" },
];

const packageJsonPath = join(rootDir, "package.json");
const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));
const version = packageJson.version;

console.log(`Building npm packages for version ${version}...`);

for (const { os, arch, binaryName, binEntry } of platforms) {
  const packageName = `@niteo/cli-${os}-${arch}`;
  const packageDir = join(npmDir, packageName);
  const binDir = join(packageDir, "bin");
  const sourceBinary = join(binariesDir, `${os}-${arch}`, binaryName);
  const destBinary = join(packageDir, binEntry);

  console.log(`\nProcessing ${packageName}...`);

  if (!existsSync(sourceBinary)) {
    console.error(`  Error: Binary not found at ${sourceBinary}`);
    process.exit(1);
  }

  mkdirSync(binDir, { recursive: true });
  copyFileSync(sourceBinary, destBinary);
  console.log(`  Copied binary to ${destBinary}`);

  const manifest = {
    name: packageName,
    version: version,
    license: "MIT",
    os: [os],
    cpu: [arch],
    bin: {
      niteo: binEntry
    },
    files: ["bin"]
  };

  const manifestPath = join(packageDir, "package.json");
  writeFileSync(manifestPath, JSON.stringify(manifest, null, 2) + "\n");
  console.log(`  Generated ${manifestPath}`);

  console.log(`  Running npm pack in ${packageDir}...`);
  execSync("npm pack", { cwd: packageDir, stdio: "inherit" });
}

console.log("\nAll platform packages packed successfully.");
