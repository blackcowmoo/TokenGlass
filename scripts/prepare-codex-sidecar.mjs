import { spawnSync } from "node:child_process";
import { constants } from "node:fs";
import { access } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { getCodexSidecarTarget } from "./codex-sidecar-target.mjs";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const binariesDir = resolve(projectRoot, "src-tauri", "binaries");
const targetTriple = getCodexSidecarTarget(process.platform, process.arch);

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
