#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PREFIX=${KALESA_PREFIX:-"$HOME/.local"}
BIN_DIR="$PREFIX/bin"
APP_DIR="$PREFIX/share/applications"

mkdir -p "$BIN_DIR" "$APP_DIR"

if [ ! -f "$SCRIPT_DIR/kalesa" ]; then
    echo "kalesa binary not found next to install.sh" >&2
    exit 1
fi

install -m 0755 "$SCRIPT_DIR/kalesa" "$BIN_DIR/kalesa"

DESKTOP_TEMPLATE="$SCRIPT_DIR/kalesa.desktop"
DESKTOP_TARGET="$APP_DIR/kalesa.desktop"

sed "s#^Exec=kalesa %f\$#Exec=$BIN_DIR/kalesa %f#" "$DESKTOP_TEMPLATE" > "$DESKTOP_TARGET"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
fi

printf '%s\n' "Kalesa installed to $BIN_DIR/kalesa"
printf '%s\n' "Desktop launcher installed to $DESKTOP_TARGET"
printf '%s\n' "You can now drag a game .exe, ELF or AppImage onto the Kalesa application launcher."
