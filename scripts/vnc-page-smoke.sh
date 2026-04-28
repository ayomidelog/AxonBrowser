#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"
set +u
source /home/ayovps/.vnc/session.env
set -u
export RUSTFLAGS='-Awarnings'
PORT=8124
RUN_ID="smoke-$(date +%s)"
SITE_DIR="$ROOT/artifacts/page-test-site"
SERVER_LOG="$ROOT/artifacts/page-test-server.log"
URL="http://127.0.0.1:${PORT}/index.html?run=${RUN_ID}"
UPLOAD_FILE="$ROOT/artifacts/upload-demo.txt"
printf 'upload demo file\n' > "$UPLOAD_FILE"
pkill chrome >/dev/null 2>&1 || true
pkill chromium >/dev/null 2>&1 || true
pkill chromium-browser >/dev/null 2>&1 || true
pkill google-chrome >/dev/null 2>&1 || true
pkill google-chrome-stable >/dev/null 2>&1 || true
sleep 2
python3 -m http.server "$PORT" --directory "$SITE_DIR" >"$SERVER_LOG" 2>&1 &
SERVER_PID=$!
cleanup() {
  kill "$SERVER_PID" >/dev/null 2>&1 || true
}
trap cleanup EXIT
PROFILE=/tmp/axonbrowser-page-smoke-$$
rm -rf "$PROFILE"
cargo run --quiet -- chrome launch "$URL" --profile "$PROFILE" --timeout-ms 15000 --poll-ms 250 >/tmp/axonbrowser-page-launch.out
cargo run --quiet -- chrome page inspect > artifacts/page-inspect.txt
cargo run --quiet -- chrome page frames > artifacts/page-frames.txt || true
cargo run --quiet -- chrome page find "Heading:Guibot Page Demo" > artifacts/page-find-heading.txt
cargo run --quiet -- chrome page focus "Text Box:Name" > artifacts/page-focus.txt
cargo run --quiet -- chrome page type "Text Box:Name" "Axon" > artifacts/page-type.txt
cargo run --quiet -- chrome page key "Text Box:Name" ctrl+a > artifacts/page-key.txt
cargo run --quiet -- chrome page type "Text Box:Name" "Axon Prime" >> artifacts/page-type.txt
cargo run --quiet -- chrome page press-enter "Button:Submit" > artifacts/page-press-enter.txt
cargo run --quiet -- chrome page wait --text "Submitted: Axon Prime" --timeout-ms 8000 --poll-ms 250 > artifacts/page-wait-text.txt
cargo run --quiet -- chrome goto "$URL" --timeout-ms 12000 --poll-ms 250 > artifacts/page-reset.txt
cargo run --quiet -- chrome page click-and-wait "Link:Open linked page" --title-contains "Linked Target" --timeout-ms 12000 --poll-ms 250 > artifacts/page-click-wait.txt
cargo run --quiet -- chrome goto "$URL" --timeout-ms 12000 --poll-ms 250 >> artifacts/page-reset.txt
cargo run --quiet -- chrome page focus "Text Box:Query" >> artifacts/page-focus.txt
cargo run --quiet -- chrome page type "Text Box:Query" "Axon Route" > artifacts/page-submit-type.txt
cargo run --quiet -- chrome page submit-and-wait "Text Box:Query" --url-contains "submitted.html" --timeout-ms 12000 --poll-ms 250 > artifacts/page-submit-wait.txt
cargo run --quiet -- chrome goto "$URL" --timeout-ms 12000 --poll-ms 250 >> artifacts/page-reset.txt
cargo run --quiet -- chrome page check "Check Box:Accept Terms" > artifacts/page-check.txt
cargo run --quiet -- chrome page check "Radio Button:Pro Plan" >> artifacts/page-check.txt
cargo run --quiet -- chrome page select-option "Combo Box:Pet" "Otter" > artifacts/page-select-option.txt
cargo run --quiet -- chrome page wait --text "Toggle status: terms:on, plan:pro, pet:Otter" --timeout-ms 8000 --poll-ms 250 > artifacts/page-wait-toggle.txt
cargo run --quiet -- chrome page upload "Button~Upload File" "$UPLOAD_FILE" > artifacts/page-upload.txt
cargo run --quiet -- chrome page wait --text "Selected file: upload-demo.txt" --timeout-ms 8000 --poll-ms 250 > artifacts/page-wait-upload.txt
cargo run --quiet -- chrome page focus --frame "Frame:Demo Frame" "Text Box:Frame Name" > artifacts/page-frame-focus.txt
cargo run --quiet -- chrome page type --frame "Frame:Demo Frame" "Text Box:Frame Name" "Nested Axon" > artifacts/page-frame-type.txt
cargo run --quiet -- chrome page click --frame "Frame:Demo Frame" "Button:Save Frame" > artifacts/page-frame-click.txt
cargo run --quiet -- chrome page wait --frame "Frame:Demo Frame" --text "Frame saved: Nested Axon" --timeout-ms 8000 --poll-ms 250 > artifacts/page-frame-wait.txt
cargo run --quiet -- chrome goto "$URL" --timeout-ms 12000 --poll-ms 250 >> artifacts/page-reset.txt
cargo run --quiet -- chrome page click "Button:Dismiss Notice" > artifacts/page-click.txt
cargo run --quiet -- chrome page wait --text "Press dismiss to hide me." --disappear --timeout-ms 8000 --poll-ms 250 > artifacts/page-wait-disappear.txt
cargo run --quiet -- chrome screenshot artifacts/page-demo.png > artifacts/page-screenshot.txt
cargo run --quiet -- chrome current --json > artifacts/page-current.json
printf 'page smoke ok\n'
