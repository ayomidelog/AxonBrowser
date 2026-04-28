#!/usr/bin/env bash
set -euo pipefail

PREFIX="${1:-$HOME/.local/opt/axonbrowser/firefox}"
WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

ARCHIVE_PATH="$WORKDIR/firefox.tar.xz"
curl -fsSL "https://download.mozilla.org/?product=firefox-latest&os=linux64&lang=en-US" -o "$ARCHIVE_PATH"

rm -rf "$PREFIX"
mkdir -p "$WORKDIR/extract"
tar -xf "$ARCHIVE_PATH" -C "$WORKDIR/extract"
mkdir -p "$(dirname "$PREFIX")"
mv "$WORKDIR/extract/firefox" "$PREFIX"

BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
cat >"$BIN_DIR/firefox" <<EOF
#!/usr/bin/env bash
exec "$PREFIX/firefox" "\$@"
EOF
chmod +x "$BIN_DIR/firefox"

echo "local firefox install ready at $PREFIX"
echo "wrapper: $BIN_DIR/firefox"
