#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
PROFILE="${GUIBOT_CHROME_FLOW_PROFILE:-/tmp/axonbrowser-vnc-flow-profile-$(date +%s)}"
ARTIFACT_DIR="${GUIBOT_ARTIFACT_DIR:-$ROOT/artifacts}"
mkdir -p "$ARTIFACT_DIR"
cleanup() {
  pkill -f "/opt/google/chrome/chrome.*$PROFILE" >/dev/null 2>&1 || true
  sleep 1
  rm -rf "$PROFILE" >/dev/null 2>&1 || true
}
trap cleanup EXIT

printf -- '--- launch ---\n'
./scripts/use-vnc-session.sh cargo run --quiet -- chrome launch about:blank --profile "$PROFILE" --timeout-ms 20000 --poll-ms 250

printf -- '\n--- attach json ---\n'
./scripts/use-vnc-session.sh cargo run --quiet -- chrome attach --json | tee "$ARTIFACT_DIR/vnc-flow-attach.json"

printf -- '\n--- current json baseline ---\n'
./scripts/use-vnc-session.sh cargo run --quiet -- chrome current --json | tee "$ARTIFACT_DIR/vnc-flow-current-baseline.json"

printf -- '\n--- goto example ---\n'
./scripts/use-vnc-session.sh cargo run --quiet -- chrome goto example.com --timeout-ms 20000 --poll-ms 250

printf -- '\n--- goto hacker news new tab ---\n'
./scripts/use-vnc-session.sh cargo run --quiet -- chrome goto https://news.ycombinator.com --new-tab --timeout-ms 25000 --poll-ms 250

printf -- '\n--- switch back to example ---\n'
./scripts/use-vnc-session.sh cargo run --quiet -- chrome tabs switch --title example

printf -- '\n--- screenshot ---\n'
./scripts/use-vnc-session.sh cargo run --quiet -- chrome screenshot "$ARTIFACT_DIR/vnc-flow-chrome.png"

printf -- '\n--- final current json ---\n'
FINAL_JSON=$(./scripts/use-vnc-session.sh cargo run --quiet -- chrome current --json)
printf '%s\n' "$FINAL_JSON" | tee "$ARTIFACT_DIR/vnc-flow-current-final.json"

python3 - <<'PY' "$ARTIFACT_DIR/vnc-flow-current-final.json"
import json
import pathlib
import sys
path = pathlib.Path(sys.argv[1])
data = json.loads(path.read_text())
url = data.get("url", "")
title = data.get("current_tab_title", "")
assert "example" in url.lower(), f"expected example url, got {url!r}"
assert "example" in title.lower(), f"expected example title, got {title!r}"
print(f"asserted final state: title={title!r}, url={url!r}")
PY

echo "artifacts:"
echo "  $ARTIFACT_DIR/vnc-flow-attach.json"
echo "  $ARTIFACT_DIR/vnc-flow-current-baseline.json"
echo "  $ARTIFACT_DIR/vnc-flow-current-final.json"
echo "  $ARTIFACT_DIR/vnc-flow-chrome.png"
