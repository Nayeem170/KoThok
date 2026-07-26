#!/bin/sh
# KoThok Installer (macOS) - downloads binary from GitHub releases
DIR="$(cd "$(dirname "$0")" && pwd)"
exec pwsh -NoProfile -File "$DIR/install.ps1" "$@"
