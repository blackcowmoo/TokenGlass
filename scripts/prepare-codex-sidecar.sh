#!/bin/sh
set -eu

binaries_dir="$1"

if [ "$(uname -s)" = "Darwin" ] && [ "$(uname -m)" = "arm64" ]; then
  target_triple="aarch64-apple-darwin"
elif [ "$(uname -s)" = "Darwin" ] && [ "$(uname -m)" = "x86_64" ]; then
  target_triple="x86_64-apple-darwin"
elif [ "$(uname -s)" = "Linux" ] && [ "$(uname -m)" = "x86_64" ]; then
  target_triple="x86_64-unknown-linux-gnu"
else
  echo "Codex sidecar preparation is not configured for $(uname -s)/$(uname -m)." >&2
  exit 1
fi

mkdir -p "$binaries_dir"
export CODEX_INSTALL_DIR="$binaries_dir"
export CODEX_NON_INTERACTIVE=1
curl -fsSL https://chatgpt.com/codex/install.sh | sh

installer_binary=""
for candidate in \
  "$binaries_dir/codex-x86_64-unknown-linux-gnu" \
  "$binaries_dir/codex-x86_64-unknown-linux-musl" \
  "$binaries_dir/codex-arm64-apple-darwin" \
  "$binaries_dir/codex-x86_64-apple-darwin" \
  "$binaries_dir/codex" \
  "$HOME/.codex/packages/standalone/current/codex" \
  "$(command -v codex 2>/dev/null || true)"; do
  if [ -n "$candidate" ] && [ -x "$candidate" ]; then
    installer_binary="$candidate"
    break
  fi
done

if [ -z "$installer_binary" ]; then
  echo "The Codex installer did not produce an executable." >&2
  exit 1
fi

if [ "$installer_binary" != "$binaries_dir/codex-$target_triple" ]; then
  cp -L "$installer_binary" "$binaries_dir/codex-$target_triple"
fi
rm -f "$binaries_dir/codex" \
      "$binaries_dir/codex-arm64-apple-darwin" \
      "$binaries_dir/codex-x86_64-unknown-linux-musl" \
      "$binaries_dir/codex-code-mode-host"
