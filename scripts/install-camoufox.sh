#!/usr/bin/env bash
set -euo pipefail

VENV_DIR="${GUIBOT_CAMOUFOX_VENV:-$HOME/.local/share/axonbrowser/camoufox-venv}"
python3 -m venv "$VENV_DIR"
"$VENV_DIR/bin/pip" install --upgrade pip
"$VENV_DIR/bin/pip" install camoufox

BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"

if "$VENV_DIR/bin/python" -m camoufox path >/dev/null 2>&1; then
  cat >"$BIN_DIR/camoufox" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
VENV_DIR="${GUIBOT_CAMOUFOX_VENV:-$HOME/.local/share/axonbrowser/camoufox-venv}"
BIN="$("$VENV_DIR/bin/python" -m camoufox path)"
if [[ -d "$BIN" ]]; then
  BIN="$BIN/camoufox"
fi
exec "$BIN" "$@"
EOF
  chmod +x "$BIN_DIR/camoufox"
fi

echo "camoufox installed in $VENV_DIR"
