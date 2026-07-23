<#
.SYNOPSIS
    Install KoThok to a Kobo e-reader. Downloads binary from GitHub releases.
.DESCRIPTION
    No source code, no Rust, no Docker needed.
    Just run this script with your Kobo plugged in via USB.
.PARAMETER Repo
    GitHub repo with releases (default: Nayeem170/KoThok).
#>
[CmdletBinding()]
param(
    [string]$Repo = "Nayeem170/KoThok"
)

$ErrorActionPreference = 'Stop'
$ApiUrl = "https://api.github.com/repos/$Repo/releases/latest"
$TempDir = Join-Path $env:TEMP 'kothok-install'

function Step($msg)  { Write-Host "`n  $msg" -ForegroundColor White }
function Ok($msg)    { Write-Host "  [OK] $msg" -ForegroundColor Green }
function Info($msg)  { Write-Host "  $msg" -ForegroundColor DarkGray }
function Warn($msg)  { Write-Host "  [!] $msg" -ForegroundColor Yellow }
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
Write-Host "  KoThok Installer" -ForegroundColor Cyan
Write-Host "  ================" -ForegroundColor Cyan

# --- 1. Download from GitHub releases ----------------------------------------
Step "Fetching latest release..."

try {
    $release = Invoke-RestMethod -Uri $ApiUrl -TimeoutSec 30 -Headers @{ 'User-Agent' = 'KoThok-Installer' }
} catch {
    Fail "Could not reach GitHub. Check internet connection.`n  $ApiUrl"
}

$version = $release.tag_name
$binaryAsset = $release.assets | Where-Object { $_.name -like 'kothok-*' } | Select-Object -First 1
$tgzAsset = $release.assets | Where-Object { $_.name -like '*.KoboRoot.tgz' } | Select-Object -First 1

if (-not $binaryAsset) {
    Fail "No binary found in release $version.`n  Expected asset: kothok-* in: https://github.com/$Repo/releases"
}

if (Test-Path $TempDir) { Remove-Item -Recurse -Force $TempDir }
New-Item -ItemType Directory -Force -Path $TempDir | Out-Null

Info "Downloading kothok $version..."
$binaryPath = Join-Path $TempDir 'kothok'
Invoke-WebRequest -Uri $binaryAsset.browser_download_url -OutFile $binaryPath -UseBasicParsing -TimeoutSec 120

if (-not (Test-Path $binaryPath)) { Fail "Download failed." }

$hash = (Get-FileHash -LiteralPath $binaryPath -Algorithm MD5).Hash
Ok "Downloaded kothok $version (MD5: $hash)"

# --- 2. Find Kobo USB drive --------------------------------------------------
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
$binaryOnDevice = Join-Path $addsDir 'kothok'

# --- 2b. Fonts ---------------------------------------------------------------
# Every script's face is installed up front so that reading never needs a
# network connection - only read-aloud does. Without these, any non-Latin book
# renders as blank boxes.
#
# Run on both paths: a first install gets them from KoboRoot.tgz, but an update
# only replaces the binary, so an older device would keep a stale font set.
function Install-Fonts($release, $addsDir, $tempDir) {
    $asset = $release.assets | Where-Object { $_.name -eq 'kothok-fonts.zip' } | Select-Object -First 1
    if (-not $asset) {
        Warn "No kothok-fonts.zip in this release - skipping font install."
        Info "Non-Latin books may render as blank boxes."
        return
    }

    $fontDir = Join-Path $addsDir 'fonts'
    New-Item -ItemType Directory -Force -Path $fontDir | Out-Null

    $zip = Join-Path $tempDir 'kothok-fonts.zip'
    Step "Installing fonts..."
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $zip -UseBasicParsing -TimeoutSec 300

    $extract = Join-Path $tempDir 'fonts'
    Expand-Archive -LiteralPath $zip -DestinationPath $extract -Force

    $copied = 0
    foreach ($f in Get-ChildItem $extract -Filter '*.ttf') {
        $dest = Join-Path $fontDir $f.Name
        # Compare by length: these are versioned releases, not edited files.
        if ((Test-Path $dest) -and ((Get-Item $dest).Length -eq $f.Length)) { continue }
        Copy-Item -LiteralPath $f.FullName -Destination $dest -Force
        $copied++
    }
    $total = (Get-ChildItem $fontDir -Filter '*.ttf').Count
    Ok "Fonts: $total installed ($copied updated)"
}

# --- 3. First install or update ----------------------------------------------
$isFirstInstall = -not (Test-Path -LiteralPath $binaryOnDevice)

if ($isFirstInstall) {
    if (-not $tgzAsset) {
        Fail @"
First install needs KoboRoot.tgz but it is not in release $version.
Download it manually: https://github.com/$Repo/releases
Copy it to the Kobo USB root, eject, and reboot.
"@
    }

    Step "First install - downloading KoboRoot.tgz..."
    $tgzPath = Join-Path $TempDir $tgzAsset.name
    Invoke-WebRequest -Uri $tgzAsset.browser_download_url -OutFile $tgzPath -UseBasicParsing -TimeoutSec 120
    Copy-Item -LiteralPath $tgzPath -Destination $koboRoot -Force
    $tgzHash = (Get-FileHash -LiteralPath $tgzPath -Algorithm MD5).Hash

    # Fonts ship inside the tgz, but extract only after the reboot. Top them up
    # from kothok-fonts.zip now too, so every face is in .adds/fonts the moment
    # the install finishes - even if a tgz ever shipped without them.
    Install-Fonts $release $addsDir $TempDir

    Write-Host ""
    Write-Host "  ============================" -ForegroundColor Green
    Write-Host "  FIRST INSTALL COMPLETE" -ForegroundColor Green
    Write-Host "  ============================" -ForegroundColor Green
    Info "Version: $version"
    Info "MD5: $tgzHash"
    Write-Host ""
    Write-Host "  DONE! Follow these steps:" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "  1. Eject the Kobo (system tray -> Safely Remove -> KOBOeReader)" -ForegroundColor White
    Write-Host "  2. Unplug USB cable" -ForegroundColor White
    Write-Host "  3. Reboot the Kobo (hold power 15s, release, press again)" -ForegroundColor White
    Write-Host "  4. Wait for 'Updating...' to finish (~30s)" -ForegroundColor White
    Write-Host "  5. Tap the hamburger menu (bottom-right) -> tap 'KoThok'" -ForegroundColor White
    Write-Host ""
}
else {
    Step "Update - copying binary..."
    Copy-Item -LiteralPath $binaryPath -Destination $binaryOnDevice -Force
    $devHash = (Get-FileHash -LiteralPath $binaryOnDevice -Algorithm MD5).Hash.ToLower()

    if ($hash.ToLower() -ne $devHash) {
        Fail "MD5 mismatch! download=$hash device=$devHash"
    }

    Install-Fonts $release $addsDir $TempDir

    Write-Host ""
    Write-Host "  ============================" -ForegroundColor Green
    Write-Host "  UPDATE COMPLETE" -ForegroundColor Green
    Write-Host "  ============================" -ForegroundColor Green
    Info "Version: $version"
    Info "MD5: $hash"
    Write-Host ""
    Write-Host "  DONE! Follow these steps:" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "  1. Eject the Kobo (system tray -> Safely Remove -> KOBOeReader)" -ForegroundColor White
    Write-Host "  2. Unplug USB cable" -ForegroundColor White
    Write-Host "  3. Tap the hamburger menu (bottom-right) -> tap 'KoThok'" -ForegroundColor White
    Write-Host ""
}

Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
