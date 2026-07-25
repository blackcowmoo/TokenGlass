#!/bin/sh
set -eu

binaries_dir="$1"
target_triple="aarch64-apple-darwin"

if [ "$(uname -s)" != "Darwin" ] || [ "$(uname -m)" != "arm64" ]; then
  echo "Codex sidecar preparation is currently configured for Apple Silicon macOS only." >&2
  exit 1
fi

mkdir -p "$binaries_dir"
export CODEX_INSTALL_DIR="$binaries_dir"
export CODEX_NON_INTERACTIVE=1
curl -fsSL https://chatgpt.com/codex/install.sh | sh

installer_binary="$binaries_dir/codex-arm64-apple-darwin"
if [ ! -x "$installer_binary" ] && [ -x "$binaries_dir/codex" ]; then
  installer_binary="$binaries_dir/codex"
fi

if [ ! -x "$installer_binary" ]; then
  echo "The Codex installer did not produce an executable." >&2
  exit 1
fi

cp -L "$installer_binary" "$binaries_dir/codex-$target_triple"
rm -f "$binaries_dir/codex" "$binaries_dir/codex-arm64-apple-darwin" "$binaries_dir/codex-code-mode-host"
