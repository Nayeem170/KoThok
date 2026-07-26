@echo off
title KoThok Uninstaller
pwsh -ExecutionPolicy Bypass -NoProfile -File "%~dp0uninstall.ps1" %*
echo.
set /p dummy=  Press Enter to close...
