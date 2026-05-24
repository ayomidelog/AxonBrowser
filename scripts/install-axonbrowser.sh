#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
mkdir -p "$BIN_DIR"

if [[ -x "$ROOT/bin/axonbrowser" ]]; then
  cp "$ROOT/bin/axonbrowser" "$BIN_DIR/axonbrowser"
elif [[ -x "$ROOT/target/release/axonbrowser" ]]; then
  cp "$ROOT/target/release/axonbrowser" "$BIN_DIR/axonbrowser"
else
  cargo build --release
  cp "$ROOT/target/release/axonbrowser" "$BIN_DIR/axonbrowser"
fi

chmod +x "$BIN_DIR/axonbrowser"

"$ROOT/scripts/install-runtime-deps.sh"

if [[ "${GUIBOT_INSTALL_CAMOUFOX:-1}" == "1" ]]; then
  "$ROOT/scripts/install-camoufox.sh"
fi

if [[ "${GUIBOT_INSTALL_CHROME:-1}" == "1" ]]; then
  "$ROOT/scripts/install-chrome-local.sh"
fi

if [[ "${GUIBOT_INSTALL_FIREFOX:-1}" == "1" ]]; then
  "$ROOT/scripts/install-firefox-local.sh"
fi

if [[ "${GUIBOT_INSTALL_EDGE:-1}" == "1" ]]; then
  "$ROOT/scripts/install-edge-local.sh"
fi

echo "installed axonbrowser into $BIN_DIR"
echo "ensure $BIN_DIR is on PATH"
