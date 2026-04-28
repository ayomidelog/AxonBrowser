#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
PROFILE="${GUIBOT_CHROME_PROFILE:-/tmp/axonbrowser-vnc-chrome-profile}"
ARTIFACT_DIR="${GUIBOT_ARTIFACT_DIR:-$ROOT/artifacts}"
mkdir -p "$ARTIFACT_DIR"
pkill -f "/opt/google/chrome/chrome.*$PROFILE" >/dev/null 2>&1 || true
rm -rf "$PROFILE"
./scripts/use-vnc-session.sh nohup google-chrome \
  --user-data-dir="$PROFILE" \
  --no-first-run \
  --no-default-browser-check \
  --disable-search-engine-choice-screen \
  --force-renderer-accessibility \
  --new-window \
  --window-size=1280,900 \
  about:blank > /tmp/axonbrowser-vnc-smoke-chrome.log 2>&1 &
CHROME_PID=$!
sleep 5
./scripts/use-vnc-session.sh cargo run --quiet -- wait window chrome --timeout-ms 15000 --poll-ms 250
./scripts/use-vnc-session.sh cargo run --quiet -- chrome locate address-bar
./scripts/use-vnc-session.sh cargo run --quiet -- chrome screenshot "$ARTIFACT_DIR/vnc-smoke-chrome.png"
echo "chrome pid: $CHROME_PID"
echo "artifact: $ARTIFACT_DIR/vnc-smoke-chrome.png"
