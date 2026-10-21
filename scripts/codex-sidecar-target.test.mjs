import assert from "node:assert/strict";
import test from "node:test";
import { getCodexSidecarTarget } from "./codex-sidecar-target.mjs";

test("selects the Intel macOS Codex sidecar target", () => {
  assert.equal(getCodexSidecarTarget("darwin", "x64"), "x86_64-apple-darwin");
});

test("preserves existing supported Codex sidecar targets", () => {
  assert.equal(getCodexSidecarTarget("darwin", "arm64"), "aarch64-apple-darwin");
  assert.equal(getCodexSidecarTarget("linux", "x64"), "x86_64-unknown-linux-gnu");
  assert.equal(getCodexSidecarTarget("win32", "arm64"), "x86_64-pc-windows-msvc");
});

test("rejects unsupported platform and architecture pairs", () => {
  assert.equal(getCodexSidecarTarget("darwin", "ia32"), null);
  assert.equal(getCodexSidecarTarget("linux", "arm64"), null);
  assert.equal(getCodexSidecarTarget("freebsd", "x64"), null);
});
