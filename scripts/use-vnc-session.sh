#!/usr/bin/env bash
set -euo pipefail
SESSION_ENV="${GUIBOT_VNC_SESSION_ENV:-$HOME/.cache/axonbrowser/headless/session.env}"
if [[ ! -f "$SESSION_ENV" ]]; then
  echo "missing VNC session env: $SESSION_ENV" >&2
  exit 1
fi
set +u
# shellcheck disable=SC1090
source "$SESSION_ENV"
set -u
export DISPLAY XDG_SESSION_TYPE NO_AT_BRIDGE GTK_MODULES QT_LINUX_ACCESSIBILITY_ALWAYS_ON ACCESSIBILITY_ENABLED GNOME_ACCESSIBILITY DBUS_SESSION_BUS_ADDRESS DBUS_SESSION_BUS_PID
exec "$@"
