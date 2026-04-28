#!/usr/bin/env bash
set -euo pipefail

PREFIX="${1:-$HOME/.local/opt/axonbrowser/microsoft-edge}"
WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

PACKAGE_INDEX_URL="https://packages.microsoft.com/repos/edge/dists/stable/main/binary-amd64/Packages.gz"
BASE_URL="https://packages.microsoft.com/repos/edge"

filename="$(
  curl -fsSL "$PACKAGE_INDEX_URL" \
    | gzip -dc \
    | awk '
        /^Package: microsoft-edge-stable$/ { pkg=1; next }
        pkg && /^Filename:/ { latest=$2; pkg=0 }
        END { print latest }
      '
)"

if [[ -z "$filename" ]]; then
  echo "install-edge-local: failed to resolve the latest microsoft-edge-stable package" >&2
  exit 1
fi

deb_path="$WORKDIR/edge.deb"
curl -fsSL "$BASE_URL/$filename" -o "$deb_path"

pkg_dir="$WORKDIR/pkg"
mkdir -p "$pkg_dir"
cd "$pkg_dir"
ar x "$deb_path"
tar -xf data.tar.xz

rm -rf "$PREFIX"
mkdir -p "$PREFIX"
cp -a opt/microsoft/msedge/. "$PREFIX/"

BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
cat >"$BIN_DIR/microsoft-edge" <<EOF
#!/usr/bin/env bash
exec "$PREFIX/microsoft-edge" "\$@"
EOF
chmod +x "$BIN_DIR/microsoft-edge"

echo "local edge install ready at $PREFIX"
echo "wrapper: $BIN_DIR/microsoft-edge"
