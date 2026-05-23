#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

source "${GUIBOT_VNC_SESSION_ENV:-$HOME/.cache/axonbrowser/headless/session.env}" 2>/dev/null || true

BIN="${AXONBROWSER_BIN:-$ROOT/target/release/axonbrowser}"
ARTIFACT_DIR="${GUIBOT_ARTIFACT_DIR:-$ROOT/artifacts/chrome-sweep}"
PROFILE="${GUIBOT_CHROME_SWEEP_PROFILE:-/tmp/axonbrowser-chrome-sweep-profile}"
PORT="${GUIBOT_CHROME_SWEEP_PORT:-8124}"
RUN_ID="chrome-sweep-$(date +%s)"
SITE_DIR="$ROOT/artifacts/page-test-site"
SERVER_LOG="$ARTIFACT_DIR/server.log"
URL="http://127.0.0.1:${PORT}/index.html?run=${RUN_ID}"
UPLOAD_FILE="$ARTIFACT_DIR/upload-demo.txt"

mkdir -p "$ARTIFACT_DIR"
printf 'upload demo file\n' >"$UPLOAD_FILE"

cleanup() {
  kill "${SERVER_PID:-}" >/dev/null 2>&1 || true
  pkill -f "/opt/google/chrome/chrome.*$PROFILE" >/dev/null 2>&1 || true
  rm -rf "$PROFILE" >/dev/null 2>&1 || true
}
trap cleanup EXIT

run() {
  local name="$1"
  shift
  printf -- '\n--- %s ---\n' "$name"
  "$@" | tee "$ARTIFACT_DIR/$name.txt"
}

rm -rf "$PROFILE"
fuser -k "${PORT}/tcp" >/dev/null 2>&1 || true
setsid -f python3 -m http.server "$PORT" --directory "$SITE_DIR" >"$SERVER_LOG" 2>&1
sleep 1
SERVER_PID="$(pgrep -af "python3 -m http.server $PORT --directory $SITE_DIR" | awk 'NR==1{print $1}')"

run launch "$BIN" chrome launch "$URL" --profile "$PROFILE" --timeout-ms 20000 --poll-ms 250
run windows "$BIN" chrome windows
run attach_json "$BIN" chrome attach --json
run current_json "$BIN" chrome current --json
run locate_address "$BIN" chrome locate address-bar
run focus_address "$BIN" chrome focus address-bar
run type_address "$BIN" chrome type address-bar "$URL"
run key_address "$BIN" chrome key address-bar ctrl+l
run press_enter "$BIN" chrome press-enter --locator address-bar
run hold_ctrl "$BIN" chrome hold ctrl --duration-ms 250
run wait_locator "$BIN" chrome wait locator address-bar --timeout-ms 8000 --poll-ms 250
run screenshot_window "$BIN" chrome screenshot "$ARTIFACT_DIR/window.png"
run resize_desktop "$BIN" chrome resize --preset desktop
run resize_mobile "$BIN" chrome resize --preset mobile
run resize_custom "$BIN" chrome resize --width 1280 --height 900
run tabs_list_a "$BIN" chrome tabs list
run goto_example "$BIN" chrome goto example.com --timeout-ms 12000 --poll-ms 250
run wait_url_change "$BIN" chrome wait url-change --timeout-ms 8000 --poll-ms 250
run new_tab "$BIN" chrome new-tab
run tabs_list_b "$BIN" chrome tabs list
run goto_google_new_tab "$BIN" chrome goto google.com --new-tab --timeout-ms 12000 --poll-ms 250
run tabs_switch_example "$BIN" chrome tabs switch --title example
run tabs_close_index1 "$BIN" chrome tabs close --index 1
run back "$BIN" chrome back
run forward "$BIN" chrome forward
run reload "$BIN" chrome reload
run current_text "$BIN" chrome current

run page_inspect "$BIN" chrome page inspect
run page_frames "$BIN" chrome page frames || true
run page_find_heading "$BIN" chrome page find "Heading:Guibot Page Demo"
run page_count_buttons "$BIN" chrome page count "Push Button"
run page_read_heading "$BIN" chrome page read "Heading:Guibot Page Demo"
run page_focus_name "$BIN" chrome page focus "Text Box:Name"
run page_type_name "$BIN" chrome page type "Text Box:Name" "Axon"
run page_key_name "$BIN" chrome page key "Text Box:Name" ctrl+a
run page_type_name2 "$BIN" chrome page type "Text Box:Name" "Axon Prime"
run page_press_enter "$BIN" chrome page press-enter "Button:Submit"
run page_wait_text "$BIN" chrome page wait --text "Submitted: Axon Prime" --timeout-ms 8000 --poll-ms 250
run page_reset_a "$BIN" chrome goto "$URL" --timeout-ms 12000 --poll-ms 250
run page_click_wait "$BIN" chrome page click-and-wait "Link:Open linked page" --title-contains "Linked Target" --timeout-ms 12000 --poll-ms 250
run page_reset_b "$BIN" chrome goto "$URL" --timeout-ms 12000 --poll-ms 250
run page_focus_query "$BIN" chrome page focus "Text Box:Query"
run page_type_query "$BIN" chrome page type "Text Box:Query" "Axon Route"
run page_submit_wait "$BIN" chrome page submit-and-wait "Text Box:Query" --url-contains "submitted.html" --timeout-ms 12000 --poll-ms 250
run page_reset_c "$BIN" chrome goto "$URL" --timeout-ms 12000 --poll-ms 250
run page_check_terms "$BIN" chrome page check "Check Box:Accept Terms"
run page_check_plan "$BIN" chrome page check "Radio Button:Pro Plan"
run page_select_option "$BIN" chrome page select-option "Combo Box:Pet" "Otter"
run page_wait_toggle "$BIN" chrome page wait --text "Toggle status: terms:on, plan:pro, pet:Otter" --timeout-ms 8000 --poll-ms 250
run page_upload "$BIN" chrome page upload "Button~Upload File" "$UPLOAD_FILE"
run page_wait_upload "$BIN" chrome page wait --text "Selected file: upload-demo.txt" --timeout-ms 8000 --poll-ms 250
run page_frame_focus "$BIN" chrome page focus --frame "Frame:Demo Frame" "Text Box:Frame Name"
run page_frame_type "$BIN" chrome page type --frame "Frame:Demo Frame" "Text Box:Frame Name" "Nested Axon"
run page_frame_click "$BIN" chrome page click --frame "Frame:Demo Frame" "Button:Save Frame"
run page_frame_wait "$BIN" chrome page wait --frame "Frame:Demo Frame" --text "Frame saved: Nested Axon" --timeout-ms 8000 --poll-ms 250
run page_reset_d "$BIN" chrome goto "$URL" --timeout-ms 12000 --poll-ms 250
run page_click_dismiss "$BIN" chrome page click "Button:Dismiss Notice"
run page_wait_disappear "$BIN" chrome page wait --text "Press dismiss to hide me." --disappear --timeout-ms 8000 --poll-ms 250
run page_scroll_down "$BIN" chrome page scroll --direction down --amount 1
run page_scroll_target "$BIN" chrome page scroll "Button:Submit" --into-view
run page_screenshot "$BIN" chrome page screenshot "$ARTIFACT_DIR/page.png"

printf '\nartifacts: %s\n' "$ARTIFACT_DIR"
