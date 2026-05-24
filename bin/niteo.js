#!/usr/bin/env node

const { existsSync } = require("node:fs");
const { dirname, join, resolve } = require("node:path");
const { spawnSync } = require("node:child_process");

const packageRoot = resolve(__dirname, "..");
const binaryName = process.platform === "win32" ? "niteo.exe" : "niteo";
const binaryPath = join(packageRoot, "target", "release", binaryName);

function run(command, args, options) {
  const result = spawnSync(command, args, {
    cwd: packageRoot,
    stdio: "inherit",
    ...options,
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

if (!existsSync(binaryPath)) {
  const buildResult = spawnSync("cargo", ["build", "--release"], {
    cwd: packageRoot,
    stdio: "inherit",
  });

  if (buildResult.error) {
    console.error("failed to build niteo; install Rust and Cargo, then try again");
    console.error(buildResult.error.message);
    process.exit(1);
  }

  if (buildResult.status !== 0) {
    process.exit(buildResult.status ?? 1);
  }
}

run(binaryPath, process.argv.slice(2), {
  cwd: process.cwd(),
  env: process.env,
});
