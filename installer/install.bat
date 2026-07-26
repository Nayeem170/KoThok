@echo off
title KoThok Installer
pwsh -ExecutionPolicy Bypass -NoProfile -File "%~dp0install.ps1" %*
echo.
set /p dummy=  Press Enter to close...
