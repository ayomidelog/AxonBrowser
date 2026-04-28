#!/usr/bin/env bash
set -euo pipefail

PREFIX="${1:-$HOME/.local/opt/axonbrowser/google-chrome}"
WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

DEB_URL="https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb"
DEB_PATH="$WORKDIR/google-chrome.deb"

curl -fsSL "$DEB_URL" -o "$DEB_PATH"

PKG_DIR="$WORKDIR/pkg"
mkdir -p "$PKG_DIR"
cd "$PKG_DIR"
ar x "$DEB_PATH"
tar -xf data.tar.xz

rm -rf "$PREFIX"
mkdir -p "$PREFIX"
cp -a opt/google/chrome/. "$PREFIX/"

BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
cat >"$BIN_DIR/google-chrome" <<EOF
#!/usr/bin/env bash
exec "$PREFIX/google-chrome" "\$@"
EOF
chmod +x "$BIN_DIR/google-chrome"

echo "local chrome install ready at $PREFIX"
echo "wrapper: $BIN_DIR/google-chrome"
