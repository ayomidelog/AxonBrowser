#!/usr/bin/env bash
set -euo pipefail

need_headless=0
if [[ -z "${DISPLAY:-}" ]]; then
  need_headless=1
elif ! xdpyinfo >/dev/null 2>&1; then
  need_headless=1
fi

packages=(
  at-spi2-core
  dbus-x11
  imagemagick
  python3-venv
  x11-utils
  xclip
  xdotool
)

if (( need_headless )); then
  packages+=(
    xvfb
  )
fi

if ! command -v apt-get >/dev/null 2>&1; then
  echo "install-runtime-deps: apt-get is required on this installer path" >&2
  exit 1
fi

if [[ "${EUID}" -ne 0 ]]; then
  if command -v sudo >/dev/null 2>&1; then
    sudo apt-get update
    sudo apt-get install -y "${packages[@]}"
  else
    echo "install-runtime-deps: run as root or install sudo" >&2
    exit 1
  fi
else
  apt-get update
  apt-get install -y "${packages[@]}"
fi
