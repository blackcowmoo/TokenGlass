import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";

if (process.platform !== "win32" || process.arch !== "x64") {
  throw new Error("Windows x64 테스트 빌드는 Windows x64에서만 생성할 수 있습니다.");
}

const env = { ...process.env, VITE_TOKENGLASS_TEST_MODE: "true", TOKENGLASS_TEST_MODE: "true" };
const run = (args) => {
  const result = spawnSync("pnpm", args, { stdio: "inherit", env, shell: true });
  if (result.status !== 0) process.exit(result.status ?? 1);
};

run(["prepare:sidecar"]);
run(["exec", "tauri", "build"]);

const sourceSidecar = resolve("src-tauri", "binaries", "codex-x86_64-pc-windows-msvc.exe");
const targetSidecar = resolve("src-tauri", "target", "release", "codex-x86_64-pc-windows-msvc.exe");
if (existsSync(sourceSidecar)) {
  copyFileSync(sourceSidecar, targetSidecar);
}

