#!/usr/bin/env node

const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");

const BINARY_NAME = process.platform === "win32" ? "centy.exe" : "centy";
const binaryPath = path.join(__dirname, BINARY_NAME);

if (!fs.existsSync(binaryPath)) {
  console.error("centy binary not found. Running install...");
  require("../scripts/install.js");
  process.exit(0);
}

const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: "inherit",
  env: process.env,
});

child.on("error", (err) => {
  console.error("Failed to run centy:", err.message);
  process.exit(1);
});

child.on("exit", (code) => {
  process.exit(code || 0);
});
