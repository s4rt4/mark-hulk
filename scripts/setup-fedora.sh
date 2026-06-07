#!/usr/bin/env bash
#
# Install the system dependencies needed to build and run Mark-Hulk on
# Fedora (tested on Fedora 43). Tauri 2 renders through WebKitGTK, so the
# WebKit/GTK development packages must be present before `pnpm tauri dev`
# or `pnpm tauri build` will compile the Rust shell.
#
# Usage: ./scripts/setup-fedora.sh
#
set -euo pipefail

if ! command -v dnf >/dev/null 2>&1; then
  echo "This script targets Fedora (dnf not found)." >&2
  exit 1
fi

echo "Installing Tauri build dependencies via dnf (requires sudo)..."

# Toolchain for compiling the Rust shell and native crates.
sudo dnf group install -y c-development || sudo dnf groupinstall -y "C Development Tools and Libraries"

# Tauri 2 runtime/build libraries.
sudo dnf install -y \
  webkit2gtk4.1-devel \
  gtk3-devel \
  librsvg2-devel \
  openssl-devel \
  curl wget file

# rpmbuild is needed to produce the .rpm bundle (`pnpm tauri build`).
sudo dnf install -y rpm-build

echo
echo "Done. System dependencies installed."
echo "Next:"
echo "  corepack enable pnpm   # if pnpm is not yet available"
echo "  pnpm install"
echo "  pnpm tauri dev         # native window (compiles Rust on first run)"
