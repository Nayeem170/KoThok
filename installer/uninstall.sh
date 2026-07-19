#!/bin/sh
# KoThok Uninstaller (Linux) - removes KoThok from the Kobo .adds folder
DIR="$(cd "$(dirname "$0")" && pwd)"
exec pwsh -NoProfile -File "$DIR/uninstall.ps1" "$@"
