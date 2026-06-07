#!/usr/bin/env bash
#
# Install the system dependencies for the native GTK4 build of Mark-Hulk on
# Fedora (tested on Fedora 43). Mark-Hulk is a native GTK4 / libadwaita app
# written in Rust — no web engine — so it needs the GTK4 development stack
# and a Rust toolchain.
#
# Usage: ./scripts/setup-fedora.sh
#
set -euo pipefail

if ! command -v dnf >/dev/null 2>&1; then
  echo "This script targets Fedora (dnf not found)." >&2
  exit 1
fi

echo "Installing GTK4 build dependencies via dnf (requires sudo)..."

# Toolchain for compiling Rust and the native -sys crates.
sudo dnf group install -y c-development || sudo dnf groupinstall -y "C Development Tools and Libraries"

# GTK4 native stack:
#   gtk4-devel           the GTK4 toolkit
#   libadwaita-devel     GNOME styling / adaptive widgets (AdwTabView, header bar)
#   gtksourceview5-devel the source editor widget (markdown highlighting)
sudo dnf install -y \
  gtk4-devel \
  libadwaita-devel \
  gtksourceview5-devel

echo
echo "Done. System dependencies installed."
echo "Next:"
echo "  cargo run            # build and launch the native app"
