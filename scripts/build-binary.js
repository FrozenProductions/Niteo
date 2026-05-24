#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const { resolve } = require("node:path");

const packageRoot = resolve(__dirname, "..");
const result = spawnSync("cargo", ["build", "--release"], {
  cwd: packageRoot,
  stdio: "inherit",
});

if (result.error) {
  console.error("failed to build niteo; Rust and Cargo are required for this package");
  console.error(result.error.message);
  process.exit(1);
}

if (result.signal) {
  console.error(`niteo build terminated by signal ${result.signal}`);
  process.exit(1);
}

process.exit(result.status ?? 1);
