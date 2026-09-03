export function getCodexSidecarTarget(platform, arch) {
  if (platform === "win32") {
    return "x86_64-pc-windows-msvc";
  }

  if (platform === "darwin" && arch === "arm64") {
    return "aarch64-apple-darwin";
  }

  if (platform === "darwin" && arch === "x64") {
    return "x86_64-apple-darwin";
  }

  if (platform === "linux" && arch === "x64") {
    return "x86_64-unknown-linux-gnu";
  }

  return null;
}
