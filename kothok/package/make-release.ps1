<#
.SYNOPSIS
  Builds the KoThok bootstrap KoboRoot.tgz (first-install package).

.DESCRIPTION
  Stages a device-root tree and packs it into a single KoboRoot.tgz that a
  consumer drops onto the Kobo over USB. On the next reboot the Kobo updater
  extracts it to / and KoThok appears in the NickelMenu.

  This bootstrap also installs the wireless-access daemon (kothok-ftp.sh) so
  that, after this first cable install, the desktop installer can reach the
  device over Wi-Fi (Option A).

.USAGE
  .\make-release.ps1                      # uses Cargo.toml version
  .\make-release.ps1 -Version 0.2.0

.REQUIRES
  - Release binary built:
      cross build --target armv7-unknown-linux-musleabihf --release -p kothok-app
  - NickelMenu hook at .\assets\libnm.so  (see assets\README.md)
  - WSL available (used only to set Unix exec modes + build the tarball)
#>

[CmdletBinding()]
param(
    [string]$Version
)

$ErrorActionPreference = 'Stop'
$PackageDir = $PSScriptRoot

function Convert-ToWslPath([string]$win) {
    # D:\foo\bar -> /mnt/d/foo/bar
    if ($win -match '^([A-Za-z]):[\\/](.*)$') {
        $drive = $matches[1].ToLower()
        $rest  = $matches[2] -replace '\\','/'
        return "/mnt/$drive/$rest"
    }
    return $win -replace '\\','/'
}

function Read-CargoVersion {
    $toml = Get-Content (Join-Path $PackageDir '..\Cargo.toml') -Raw
    if ($toml -match '(?m)^version\s*=\s*"([^"]+)"') { return $matches[1] }
    throw 'Could not read version from Cargo.toml'
}

function Read-BuildTag {
    # BUILD_TAG now derives from Cargo.toml via env!("CARGO_PKG_VERSION"), so
    # there is no hardcoded string to regex -- just use the version itself.
    return "v$(Read-CargoVersion)"
}

if (-not $Version) { $Version = Read-CargoVersion }
$BuildTag = Read-BuildTag
Write-Host "KoThok release: version=$Version  build=$BuildTag"

# --- resolve source files ----------------------------------------------------
$binary = Join-Path $PackageDir '..\target\armv7-unknown-linux-musleabihf\release\kothok'
$runSh  = Join-Path $PackageDir '..\run.sh'
$nmCfg  = Join-Path $PackageDir 'nm\config'
$libnm  = Join-Path $PackageDir 'assets\libnm.so'

if (-not (Test-Path -LiteralPath $binary)) {
    throw "Release binary not found: $binary`nRun: cross build --target armv7-unknown-linux-musleabihf --release -p kothok-app"
}
foreach ($f in @($runSh, $nmCfg)) {
    if (-not (Test-Path -LiteralPath $f)) { throw "Missing required file: $f" }
}
if (-not (Test-Path -LiteralPath $libnm)) {
    throw "NickelMenu hook not found: $libnm`nSee package\assets\README.md - drop libnm.so there."
}

# --- stage tree --------------------------------------------------------------
$stage = Join-Path $PackageDir '.stage'
$dist  = Join-Path $PackageDir 'dist'
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force -Path $dist | Out-Null

$dirs = @(
    'mnt\onboard\.adds\nm',
    'usr\local\Kobo\imageformats'
)
foreach ($d in $dirs) { New-Item -ItemType Directory -Force -Path (Join-Path $stage $d) | Out-Null }

Copy-Item -LiteralPath $binary -Destination (Join-Path $stage 'mnt\onboard\.adds\kothok')
Copy-Item -LiteralPath $runSh  -Destination (Join-Path $stage 'mnt\onboard\.adds\run.sh')
Copy-Item -LiteralPath $nmCfg  -Destination (Join-Path $stage 'mnt\onboard\.adds\nm\config')
Copy-Item -LiteralPath $libnm  -Destination (Join-Path $stage 'usr\local\Kobo\imageformats\libnm.so')

# --- fonts ------------------------------------------------------------------
# Every script's face ships with the install so reading never needs a network
# connection - only read-aloud does. Fetch them with scripts/fetch-fonts.ps1.
$fontSrc = Join-Path $PackageDir 'fonts'
if (-not (Test-Path $fontSrc)) {
    throw @"
No fonts staged at $fontSrc.
Run scripts\fetch-fonts.ps1 first - without them every non-Latin script renders
as blank boxes on the device.
"@
}
$fontFiles = Get-ChildItem $fontSrc -Filter '*.ttf'
if ($fontFiles.Count -eq 0) { throw "No .ttf files in $fontSrc" }

$fontDst = Join-Path $stage 'mnt\onboard\.adds\fonts'
New-Item -ItemType Directory -Force -Path $fontDst | Out-Null
foreach ($f in $fontFiles) { Copy-Item -LiteralPath $f.FullName -Destination $fontDst }
Write-Host ("  fonts: {0} face(s), {1} MB" -f $fontFiles.Count,
    [math]::Round(($fontFiles | Measure-Object Length -Sum).Sum / 1MB, 1))

$verFile = Join-Path $stage 'mnt\onboard\.adds\kothok-version'
"KoThok $Version ($BuildTag)`r`nbuild: $BuildTag`r`nbuilt: $(Get-Date -Format o)" | Set-Content -LiteralPath $verFile -NoNewline:$false

# --- verify line endings on shell scripts (LF mandatory) --------------------
foreach ($s in @('mnt\onboard\.adds\run.sh')) {
    $bytes = [IO.File]::ReadAllBytes((Join-Path $stage $s))
    if ($bytes.Length -ge 2 -and $bytes[-2] -eq 13) {
        throw "CRLF detected in staged script: $s - LF required."
    }
}

# --- build tarball via WSL (correct Unix exec modes + LF-safe) --------------
$wslStage = Convert-ToWslPath $stage
$wslDist  = Convert-ToWslPath $dist
$outName  = "KoThok-$Version.KoboRoot.tgz"

Write-Host "Packing $outName ..."
$tarCmd = @"
set -e
cd '$wslStage'
chmod 0755 mnt/onboard/.adds/kothok mnt/onboard/.adds/run.sh
mkdir -p '$wslDist'
tar czf '$wslDist/$outName' .
echo TARBUILT
"@
$result = wsl.exe -e bash -lc $tarCmd
if ($LASTEXITCODE -ne 0 -or ($result -notcontains 'TARBUILT')) {
    throw "WSL tar failed:`n$result"
}

# --- font archive ------------------------------------------------------------
# The tarball covers a first install, but an update only replaces the binary -
# so an existing device would keep whatever font set it was installed with.
# Shipping the faces separately lets the installer top up an update in place.
$fontZip = Join-Path $dist 'kothok-fonts.zip'
if (Test-Path $fontZip) { Remove-Item -LiteralPath $fontZip -Force }
Compress-Archive -Path (Join-Path $fontSrc '*.ttf') -DestinationPath $fontZip
Write-Host ("Fonts: {0} ({1} MB)" -f $fontZip,
    [math]::Round((Get-Item -LiteralPath $fontZip).Length / 1MB, 2))

# --- report ------------------------------------------------------------------
$tgz = Join-Path $dist $outName
$size = [math]::Round((Get-Item -LiteralPath $tgz).Length / 1MB, 2)
$hash = (Get-FileHash -LiteralPath $tgz -Algorithm MD5).Hash
Write-Host ""
Write-Host "Built: $tgz"
Write-Host "Size:  $size MB"
Write-Host "MD5:   $hash"
Write-Host ""
Write-Host "Next: copy $outName to the Kobo onboard root over USB, eject, reboot."
