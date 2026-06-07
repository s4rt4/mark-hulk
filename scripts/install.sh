#!/usr/bin/env bash
#
# Build Mark-Hulk in release mode and install it into the user's home
# (no root needed): the binary, the .desktop launcher, and the icon. After
# this, markdown files can be opened with Mark-Hulk from the file manager.
#
# Usage: ./scripts/install.sh
#
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Building release binary..."
cargo build --release

BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"

install -m 755 target/release/mark-hulk "$BIN_DIR/mark-hulk"
install -m 644 data/com.sarta.mark-hulk.desktop "$APP_DIR/com.sarta.mark-hulk.desktop"
install -m 644 mark-hulk.svg "$ICON_DIR/com.sarta.mark-hulk.svg"

# Refresh caches so the launcher and file association show up.
update-desktop-database "$APP_DIR" 2>/dev/null || true
gtk-update-icon-cache "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

echo
echo "Installed:"
echo "  $BIN_DIR/mark-hulk"
echo "  $APP_DIR/com.sarta.mark-hulk.desktop"
echo "  $ICON_DIR/com.sarta.mark-hulk.svg"
echo
case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) echo "Note: add $BIN_DIR to your PATH to run 'mark-hulk' from the shell." ;;
esac
echo "To make Mark-Hulk the default for .md files:"
echo "  xdg-mime default com.sarta.mark-hulk.desktop text/markdown"
