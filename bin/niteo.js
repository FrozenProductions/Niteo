#!/usr/bin/env node

const { existsSync } = require("node:fs");
const { join, resolve } = require("node:path");
const { spawnSync } = require("node:child_process");

const packageRoot = resolve(__dirname, "..");
const binaryName = process.platform === "win32" ? "niteo.exe" : "niteo";

function getPlatformKey() {
  const platform = process.platform;
  const arch = process.arch;

  const validPlatforms = {
    darwin: ["arm64", "x64"],
    linux: ["arm64", "x64"],
    win32: ["x64"],
  };

  if (validPlatforms[platform] && validPlatforms[platform].includes(arch)) {
    return `${platform}-${arch}`;
  }

  return null;
}

function getPrebuiltBinaryPath() {
  const platformKey = getPlatformKey();
  if (!platformKey) {
    return null;
  }

  return join(packageRoot, "bin", platformKey, binaryName);
}

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: process.cwd(),
    stdio: "inherit",
    env: process.env,
  });

  if (result.error) {
    console.error(result.error.message);
    process.exit(1);
  }

  if (result.signal) {
    console.error(`niteo terminated by signal ${result.signal}`);
    process.exit(1);
  }

  process.exit(result.status ?? 1);
}

function buildFromSource() {
  console.error("Building Niteo from source...");
  const buildResult = spawnSync("cargo", ["build", "--release"], {
    cwd: packageRoot,
    stdio: "inherit",
  });

  if (buildResult.error) {
    console.error("Failed to build niteo; install Rust and Cargo, then try again.");
    console.error(buildResult.error.message);
    process.exit(1);
  }

  if (buildResult.status !== 0) {
    process.exit(buildResult.status ?? 1);
  }
  
  return join(packageRoot, "target", "release", binaryName);
}

const prebuiltPath = getPrebuiltBinaryPath();

if (prebuiltPath && existsSync(prebuiltPath)) {
  run(prebuiltPath, process.argv.slice(2));
} else if (process.env.NITEO_BUILD_FROM_SOURCE === "1") {
  const sourcePath = buildFromSource();
  run(sourcePath, process.argv.slice(2));
} else {
  const platformKey = getPlatformKey();
  if (!platformKey) {
    console.error(`No prebuilt Niteo binary is available for ${process.platform}-${process.arch}.`);
    console.error("Install Rust and run with NITEO_BUILD_FROM_SOURCE=1 to build from source.");
  } else {
    console.error(`Prebuilt Niteo binary not found for ${platformKey}.`);
    console.error("Reinstall niteo-cli, or run with NITEO_BUILD_FROM_SOURCE=1 to build from source.");
  }
  process.exit(1);
}
