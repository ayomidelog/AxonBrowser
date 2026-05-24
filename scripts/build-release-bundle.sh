#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT/dist/release}"
WORK_DIR="${WORK_DIR:-$ROOT/dist/release-work}"
TARGET_DIR="${TARGET_DIR:-$ROOT/target/release}"
BUNDLE_NAME="${BUNDLE_NAME:-axonbrowser-linux-x86_64}"
BUILD_BINARIES="${BUILD_BINARIES:-1}"

if [[ "$BUILD_BINARIES" == "1" ]]; then
  cargo build --release --bins
fi

rm -rf "$WORK_DIR"
mkdir -p "$WORK_DIR/bin" "$WORK_DIR/scripts" "$OUT_DIR"

cp "$TARGET_DIR/axonbrowser" "$WORK_DIR/bin/"
cp "$ROOT/README.md" "$WORK_DIR/"
cp "$ROOT/install.sh" "$WORK_DIR/"
cp "$ROOT/scripts/install-axonbrowser.sh" "$WORK_DIR/scripts/"
cp "$ROOT/scripts/install-runtime-deps.sh" "$WORK_DIR/scripts/"
cp "$ROOT/scripts/install-chrome-local.sh" "$WORK_DIR/scripts/"
cp "$ROOT/scripts/install-firefox-local.sh" "$WORK_DIR/scripts/"
cp "$ROOT/scripts/install-edge-local.sh" "$WORK_DIR/scripts/"
cp "$ROOT/scripts/install-camoufox.sh" "$WORK_DIR/scripts/"

tar -czf "$OUT_DIR/${BUNDLE_NAME}.tar.gz" -C "$WORK_DIR" .
(
  cd "$OUT_DIR"
  sha256sum "${BUNDLE_NAME}.tar.gz" > "${BUNDLE_NAME}.tar.gz.sha256"
)
cp "$ROOT/install.sh" "$OUT_DIR/install.sh"
