#!/usr/bin/env node

const https = require("https");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");
const zlib = require("zlib");

const REPO = "tupe12334/centy-installer";
const BINARY_NAME = "centy";

function getPlatform() {
  const platform = process.platform;
  const arch = process.arch;

  let os;
  switch (platform) {
    case "darwin":
      os = "apple-darwin";
      break;
    case "linux":
      os = "unknown-linux-gnu";
      break;
    case "win32":
      os = "pc-windows-msvc";
      break;
    default:
      throw new Error(`Unsupported platform: ${platform}`);
  }

  let archName;
  switch (arch) {
    case "x64":
      archName = "x86_64";
      break;
    case "arm64":
      archName = "aarch64";
      break;
    default:
      throw new Error(`Unsupported architecture: ${arch}`);
  }

  const ext = platform === "win32" ? "zip" : "tar.gz";

  return { os, arch: archName, ext };
}

function httpsGet(url) {
  return new Promise((resolve, reject) => {
    https
      .get(url, { headers: { "User-Agent": "centy-installer" } }, (res) => {
        if (res.statusCode === 302 || res.statusCode === 301) {
          return httpsGet(res.headers.location).then(resolve).catch(reject);
        }
        if (res.statusCode !== 200) {
          reject(new Error(`HTTP ${res.statusCode}: ${url}`));
          return;
        }
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      })
      .on("error", reject);
  });
}

async function getLatestVersion() {
  const url = `https://api.github.com/repos/${REPO}/releases/latest`;
  const data = await httpsGet(url);
  const release = JSON.parse(data.toString());
  return release.tag_name;
}

async function downloadBinary(version, platform) {
  const { os, arch, ext } = platform;
  const fileName = `${BINARY_NAME}-${arch}-${os}.${ext}`;
  const url = `https://github.com/${REPO}/releases/download/${version}/${fileName}`;

  console.log(`Downloading ${BINARY_NAME} ${version} for ${arch}-${os}...`);
  const data = await httpsGet(url);
  return data;
}

function extractTarGz(buffer, destDir) {
  const tempTar = path.join(destDir, "temp.tar");
  const gunzipped = zlib.gunzipSync(buffer);
  fs.writeFileSync(tempTar, gunzipped);

  // Use tar command to extract
  execSync(`tar -xf "${tempTar}" -C "${destDir}"`, { stdio: "inherit" });
  fs.unlinkSync(tempTar);
}

function extractZip(buffer, destDir) {
  const tempZip = path.join(destDir, "temp.zip");
  fs.writeFileSync(tempZip, buffer);

  // Use unzip or powershell depending on platform
  if (process.platform === "win32") {
    execSync(
      `powershell -command "Expand-Archive -Path '${tempZip}' -DestinationPath '${destDir}'"`,
      { stdio: "inherit" }
    );
  } else {
    execSync(`unzip -o "${tempZip}" -d "${destDir}"`, { stdio: "inherit" });
  }
  fs.unlinkSync(tempZip);
}

async function main() {
  try {
    const platform = getPlatform();
    const version = await getLatestVersion();

    const binDir = path.join(__dirname, "..", "bin");
    fs.mkdirSync(binDir, { recursive: true });

    const binaryPath = path.join(
      binDir,
      process.platform === "win32" ? `${BINARY_NAME}.exe` : BINARY_NAME
    );

    // Download and extract
    const data = await downloadBinary(version, platform);

    if (platform.ext === "tar.gz") {
      extractTarGz(data, binDir);
    } else {
      extractZip(data, binDir);
    }

    // Make executable on Unix
    if (process.platform !== "win32") {
      fs.chmodSync(binaryPath, 0o755);
    }

    console.log(`Successfully installed ${BINARY_NAME} ${version}`);
  } catch (error) {
    console.error("Failed to install centy:", error.message);
    console.error(
      "You can manually install from: https://github.com/tupe12334/centy-installer/releases"
    );
    process.exit(1);
  }
}

main();
