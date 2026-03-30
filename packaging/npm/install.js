#!/usr/bin/env node
const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const https = require("https");
const { createWriteStream, mkdirSync } = fs;

const REPO = "teddytennant/nous";
const VERSION = require("./package.json").version;
const TAG = `v${VERSION}`;

const TARGETS = {
  "linux-x64": "x86_64-unknown-linux-gnu",
  "linux-arm64": "aarch64-unknown-linux-gnu",
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
  "win32-x64": "x86_64-pc-windows-msvc",
};

function getPlatformKey() {
  return `${process.platform}-${process.arch}`;
}

function download(url) {
  return new Promise((resolve, reject) => {
    const follow = (url) => {
      https
        .get(url, { headers: { "User-Agent": "nous-npm-installer" } }, (res) => {
          if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
            follow(res.headers.location);
            return;
          }
          if (res.statusCode !== 200) {
            reject(new Error(`Download failed: HTTP ${res.statusCode} for ${url}`));
            return;
          }
          const chunks = [];
          res.on("data", (chunk) => chunks.push(chunk));
          res.on("end", () => resolve(Buffer.concat(chunks)));
          res.on("error", reject);
        })
        .on("error", reject);
    };
    follow(url);
  });
}

async function main() {
  const key = getPlatformKey();
  const target = TARGETS[key];

  if (!target) {
    console.error(`Unsupported platform: ${key}`);
    console.error(`Supported: ${Object.keys(TARGETS).join(", ")}`);
    console.error("Install from source: cargo install --git https://github.com/teddytennant/nous --bin nous");
    process.exit(1);
  }

  const isWindows = process.platform === "win32";
  const ext = isWindows ? "zip" : "tar.gz";
  const archive = `nous-${TAG}-${target}.${ext}`;
  const url = `https://github.com/${REPO}/releases/download/${TAG}/${archive}`;

  console.log(`Downloading nous ${TAG} for ${target}...`);

  const binDir = path.join(__dirname, "bin");
  mkdirSync(binDir, { recursive: true });

  const tmpDir = path.join(__dirname, ".tmp");
  mkdirSync(tmpDir, { recursive: true });

  const archivePath = path.join(tmpDir, archive);
  const data = await download(url);
  fs.writeFileSync(archivePath, data);

  if (isWindows) {
    execSync(`powershell -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${tmpDir}' -Force"`, {
      stdio: "inherit",
    });
  } else {
    execSync(`tar xzf "${archivePath}" -C "${tmpDir}"`, { stdio: "inherit" });
  }

  const extractedDir = path.join(tmpDir, `nous-${TAG}-${target}`);
  const binSuffix = isWindows ? ".exe" : "";

  for (const bin of ["nous", "nous-api"]) {
    const src = path.join(extractedDir, `${bin}${binSuffix}`);
    const dest = path.join(binDir, `${bin}${binSuffix}`);
    fs.copyFileSync(src, dest);
    if (!isWindows) {
      fs.chmodSync(dest, 0o755);
    }
  }

  // Write launcher scripts
  if (!isWindows) {
    for (const bin of ["nous", "nous-api"]) {
      const launcher = path.join(binDir, bin);
      // Binary is already there, just ensure executable
      fs.chmodSync(launcher, 0o755);
    }
  }

  // Cleanup
  fs.rmSync(tmpDir, { recursive: true, force: true });

  console.log(`Installed nous ${TAG} successfully.`);
}

main().catch((err) => {
  console.error("Installation failed:", err.message);
  console.error("Install from source: cargo install --git https://github.com/teddytennant/nous --bin nous");
  process.exit(1);
});
