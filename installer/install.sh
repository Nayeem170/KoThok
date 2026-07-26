#!/bin/sh
# KoThok Installer (Linux) - downloads binary from GitHub releases
DIR="$(cd "$(dirname "$0")" && pwd)"
exec pwsh -NoProfile -File "$DIR/install.ps1" "$@"
