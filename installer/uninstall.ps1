<#
.SYNOPSIS
    Uninstall KoThok from a Kobo e-reader.
.DESCRIPTION
    Deletes KoThok's own files from the .adds folder on the Kobo onboard drive:
    binary, launcher, KoThok's single NickelMenu entry, caches, and logs.
    Other mods' NickelMenu entries (.adds/nm/) and libnm.so in the root
    filesystem are left alone - both are shared infrastructure.
.PARAMETER Force
    Skip the confirmation prompt. Use with care.
#>
[CmdletBinding()]
param(
    [switch]$Force
)

$ErrorActionPreference = 'Stop'

function Step($msg)  { Write-Host "`n  $msg" -ForegroundColor White }
function Ok($msg)    { Write-Host "  [OK] $msg" -ForegroundColor Green }
function Info($msg)  { Write-Host "  $msg" -ForegroundColor DarkGray }
function Warn($msg)  { Write-Host "  [!]  $msg" -ForegroundColor Yellow }
function Fail($msg)  {
    Write-Host ""
    Write-Host "  $msg" -ForegroundColor Red
    Write-Host ""
    exit 1
}
function Find-Kobo {
    if ($IsWindows -or $PSVersionTable.Platform -ne 'Unix') {
        $d = Get-PSDrive -PSProvider FileSystem | Where-Object {
            (Test-Path (Join-Path $_.Root '.adds')) -or (Test-Path (Join-Path $_.Root '.kobo'))
        } | Select-Object -First 1
        if ($d) { return $d.Root }
    } else {
        foreach ($pattern in '/media/*/*', '/run/media/*/*', '/Volumes/*', '/mnt/*') {
            foreach ($c in (Get-Item -Path $pattern -ErrorAction SilentlyContinue)) {
                if ((Test-Path "$($c.FullName)/.adds") -or (Test-Path "$($c.FullName)/.kobo")) {
                    return $c.FullName
                }
            }
        }
    }
    return $null
}

Write-Host ""
Write-Host "  KoThok Uninstaller" -ForegroundColor Cyan
Write-Host "  ==================" -ForegroundColor Cyan

# --- 1. Find Kobo USB drive --------------------------------------------------
Step "Looking for Kobo..."
$koboRoot = Find-Kobo

if (-not $koboRoot) {
    Write-Host ""
    Write-Host "  Plug in your Kobo via USB now." -ForegroundColor Yellow
    for ($i = 30; $i -gt 0; $i--) {
        Write-Host -NoNewline "`r  Waiting for device... ${i}s  "
        Start-Sleep -Seconds 1
        $koboRoot = Find-Kobo
        if ($koboRoot) { break }
    }
    Write-Host ""
    if (-not $koboRoot) {
        Fail "Kobo not detected after 30s. Make sure it is connected and unlocked."
    }
}

Ok "Found Kobo at $koboRoot"

$addsDir = Join-Path $koboRoot '.adds'
if (-not (Test-Path -LiteralPath $addsDir)) {
    Fail "No .adds folder on this device. Nothing to uninstall."
}

# --- 2. Scan for KoThok files ------------------------------------------------
Step "Scanning for KoThok files..."

$targets = @(
    @{ Path = (Join-Path $addsDir 'kothok');         Type = 'dir';  Label = 'binary' },
    @{ Path = (Join-Path $addsDir 'run.sh');         Type = 'file'; Label = 'launcher' },
    @{ Path = (Join-Path $addsDir 'nm/config');      Type = 'file'; Label = 'KoThok menu entry' },
    @{ Path = (Join-Path $addsDir 'kothok-version'); Type = 'file'; Label = 'version marker' },
    @{ Path = (Join-Path $addsDir 'cache');          Type = 'dir';  Label = 'app cache' },
    @{ Path = (Join-Path $addsDir 'bookcache');      Type = 'dir';  Label = 'book cache' },
    @{ Path = (Join-Path $addsDir 'kothok.log');     Type = 'file'; Label = 'log' },
    @{ Path = (Join-Path $addsDir 'kothok.err');     Type = 'file'; Label = 'error log' }
)

$found = @()
$missing = @()
foreach ($t in $targets) {
    if (Test-Path -LiteralPath $t.Path) {
        $found += $t
        Info "found   $($t.Label): $($t.Path)"
    } else {
        $missing += $t
        Info "missing $($t.Label): $($t.Path)"
    }
}

if ($found.Count -eq 0) {
    Write-Host ""
    Write-Host "  No KoThok files found. Already clean." -ForegroundColor Green
    Write-Host ""
    exit 0
}

# --- 3. Confirm --------------------------------------------------------------
if (-not $Force) {
    Write-Host ""
    $answer = Read-Host "  Delete these $($found.Count) item(s)? Type 'yes' to confirm"
    if ($answer -ne 'yes') {
        Write-Host ""
        Write-Host "  Aborted. Nothing was deleted." -ForegroundColor Yellow
        Write-Host ""
        exit 0
    }
}

# --- 4. Delete ---------------------------------------------------------------
Step "Deleting..."
foreach ($t in $found) {
    try {
        Remove-Item -LiteralPath $t.Path -Recurse -Force
        Ok "deleted $($t.Label)"
    } catch {
        Warn "could not delete $($t.Label): $_"
    }
}

# --- 5. Summary --------------------------------------------------------------
Write-Host ""
Write-Host "  ============================" -ForegroundColor Green
Write-Host "  UNINSTALL COMPLETE" -ForegroundColor Green
Write-Host "  ============================" -ForegroundColor Green
Info "Removed: $($found.Count) item(s)"
if ($missing.Count -gt 0) {
    Info "Already absent: $($missing.Count) item(s) (ignored)"
}
Write-Host ""
Write-Host "  DONE! Follow these steps:" -ForegroundColor Yellow
Write-Host ""
Write-Host "  1. Eject the Kobo (system tray -> Safely Remove -> KOBOeReader)" -ForegroundColor White
Write-Host "  2. Unplug USB cable" -ForegroundColor White
Write-Host "  3. Reboot the Kobo (hold power 15s, release, press again)" -ForegroundColor White
Write-Host ""
Write-Host "  After reboot the 'KoThok' entry is gone from nickel's menu." -ForegroundColor DarkGray
Write-Host ""
Write-Host "  Preserved (not deleted):" -ForegroundColor DarkGray
Write-Host "    .adds/book.epub  - your loaded book" -ForegroundColor DarkGray
Write-Host "    .adds/positions  - your reading position" -ForegroundColor DarkGray
Write-Host "    .adds/nm/        - other mods' NickelMenu entries (KOReader, Plato, ...)" -ForegroundColor DarkGray
Write-Host "    libnm.so         - shared NickelMenu library in the root filesystem" -ForegroundColor DarkGray
Write-Host ""
