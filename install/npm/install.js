#!/usr/bin/env node
// rguard npm postinstall — downloads the platform-specific binary.
"use strict";

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const https = require("https");
const http = require("http");

const REPO = "moabualruz/rice-guard"; // TODO: update to real GitHub owner
const VERSION = require("./package.json").version;
const BIN_DIR = path.join(__dirname, "bin");

const TARGETS = {
  "darwin-x64": { target: "x86_64-apple-darwin", ext: "tar.gz" },
  "darwin-arm64": { target: "aarch64-apple-darwin", ext: "tar.gz" },
  "linux-x64": { target: "x86_64-unknown-linux-musl", ext: "tar.gz" },
  "linux-arm64": { target: "aarch64-unknown-linux-musl", ext: "tar.gz" },
  "win32-x64": { target: "x86_64-pc-windows-msvc", ext: "zip" },
};

function getPlatformKey() {
  return `${process.platform}-${process.arch}`;
}

function download(url) {
  return new Promise((resolve, reject) => {
    const get = url.startsWith("https") ? https.get : http.get;
    get(url, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        return download(res.headers.location).then(resolve, reject);
      }
      if (res.statusCode !== 200) {
        return reject(new Error(`HTTP ${res.statusCode} for ${url}`));
      }
      const chunks = [];
      res.on("data", (c) => chunks.push(c));
      res.on("end", () => resolve(Buffer.concat(chunks)));
      res.on("error", reject);
    }).on("error", reject);
  });
}

async function main() {
  const key = getPlatformKey();
  const info = TARGETS[key];
  if (!info) {
    console.error(`Unsupported platform: ${key}`);
    process.exit(1);
  }

  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/rguard-${info.target}.${info.ext}`;
  console.log(`Downloading rguard for ${info.target}...`);

  const data = await download(url);
  fs.mkdirSync(BIN_DIR, { recursive: true });

  if (info.ext === "tar.gz") {
    const tmpFile = path.join(BIN_DIR, "rguard.tar.gz");
    fs.writeFileSync(tmpFile, data);
    execSync(`tar xzf "${tmpFile}" -C "${BIN_DIR}"`, { stdio: "ignore" });
    fs.unlinkSync(tmpFile);
    fs.chmodSync(path.join(BIN_DIR, "rguard"), 0o755);
  } else {
    // zip — extract on Windows
    const tmpFile = path.join(BIN_DIR, "rguard.zip");
    fs.writeFileSync(tmpFile, data);
    execSync(`powershell -Command "Expand-Archive -Path '${tmpFile}' -DestinationPath '${BIN_DIR}' -Force"`, { stdio: "ignore" });
    fs.unlinkSync(tmpFile);
  }

  console.log("rguard installed successfully.");
}

main().catch((err) => {
  console.error("Failed to install rguard:", err.message);
  process.exit(1);
});
