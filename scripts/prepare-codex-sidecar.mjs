import { access } from "node:fs/promises";
import { constants } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const binariesDir = resolve(projectRoot, "src-tauri", "binaries");
const targetTriple =
  process.platform === "win32"
    ? "x86_64-pc-windows-msvc"
    : process.platform === "darwin" && process.arch === "arm64"
      ? "aarch64-apple-darwin"
      : process.platform === "linux" && process.arch === "x64"
        ? "x86_64-unknown-linux-gnu"
        : null;

if (!targetTriple) {
  throw new Error(
    `Codex sidecar preparation is not configured for ${process.platform}/${process.arch}.`,
  );
}

const extension = process.platform === "win32" ? ".exe" : "";
const sidecarPath = resolve(binariesDir, `codex-${targetTriple}${extension}`);

try {
  await access(sidecarPath, constants.X_OK);
} catch {
  const isWindows = process.platform === "win32";
  const installer = resolve(
    projectRoot,
    "scripts",
    isWindows ? "prepare-codex-sidecar.ps1" : "prepare-codex-sidecar.sh",
  );
  const command = isWindows ? "powershell.exe" : "/bin/sh";
  const args = isWindows
    ? ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", installer, binariesDir]
    : [installer, binariesDir];
  const result = spawnSync(command, args, { stdio: "inherit" });
  if (result.status !== 0) {
    throw new Error("Could not download the Codex sidecar.");
  }
  await access(sidecarPath, constants.X_OK);
}
