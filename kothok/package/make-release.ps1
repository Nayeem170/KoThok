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

$sampleSrc = Join-Path $PackageDir '..\samples'
if (Test-Path -LiteralPath $sampleSrc) {
    $sampleDir = Join-Path $stage 'mnt\onboard\books'
    New-Item -ItemType Directory -Force -Path $sampleDir | Out-Null
    foreach ($epub in (Get-ChildItem -LiteralPath $sampleSrc -Filter '*.epub')) {
        Copy-Item -LiteralPath $epub.FullName -Destination $sampleDir
    }
    Write-Host "  samples: bundled for fresh-install fallback"
}

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

$tgz = Join-Path $dist $outName

# --- font archive ------------------------------------------------------------
# The tarball covers a first install, but an update only replaces the binary -
# so an existing device would keep whatever font set it was installed with.
# Shipping the faces separately lets the installer top up an update in place.
$fontZip = Join-Path $dist 'kothok-fonts.zip'
if (Test-Path $fontZip) { Remove-Item -LiteralPath $fontZip -Force }
Compress-Archive -Path (Join-Path $fontSrc '*.ttf') -DestinationPath $fontZip
Write-Host ("Fonts: {0} ({1} MB)" -f $fontZip,
    [math]::Round((Get-Item -LiteralPath $fontZip).Length / 1MB, 2))

# --- sample book -------------------------------------------------------------
# Uploaded as a separate release asset so the installer can drop it on a device
# that has no books yet. Also bundled inside the tgz for first-install.
$sampleSrc = Join-Path $PackageDir '..\samples\en-sample.epub'
if (Test-Path -LiteralPath $sampleSrc) {
    $sampleDst = Join-Path $dist 'en-sample.epub'
    Copy-Item -LiteralPath $sampleSrc -Destination $sampleDst -Force
    Write-Host ("Sample: {0} ({1} KB)" -f $sampleDst,
        [math]::Round((Get-Item -LiteralPath $sampleDst).Length / 1KB))
}

# --- manual-install zip -------------------------------------------------------
# The tgz above is already a complete, self-sufficient first-install package -
# binary, run.sh, NickelMenu hook, fonts, sample book. Anyone who'd rather not
# run install.ps1 (no PowerShell 7, or just doesn't want to run a script) can
# drag it onto the device by hand instead. Pre-renaming it here removes the
# most common way that goes wrong: Windows hides file extensions by default,
# so a fumbled manual rename silently becomes "KoboRoot.tgz.txt".
$manualStage = Join-Path $PackageDir '.manual-stage'
if (Test-Path $manualStage) { Remove-Item -Recurse -Force $manualStage }
New-Item -ItemType Directory -Force -Path $manualStage | Out-Null

Copy-Item -LiteralPath (Join-Path $dist $outName) -Destination (Join-Path $manualStage 'KoboRoot.tgz')

$instructions = @"
KoThok $Version - manual install (no script needed)

1. Plug your Kobo into your computer with a USB cable.
2. Copy KoboRoot.tgz (in this zip) onto the Kobo, into the ".kobo" folder
   (it's next to a folder called ".adds" - both are hidden folders, so make
   sure your file browser is set to show hidden files).
3. Eject / safely remove the Kobo, then unplug the USB cable.
4. Hold the power button for about 30 seconds, until the screen goes
   completely blank.
5. Wait 10 seconds, then press the power button once to turn it back on.
6. Watch for an "Updating..." screen for about 30 seconds - this is normal,
   it's installing KoThok's menu button. Do not unplug or power off here.
7. Once the device finishes booting, tap the menu button (bottom-right),
   then tap "KoThok" to open it.

Fonts for every supported script and a sample book are already included -
nothing else to copy.

Updating later: repeat these same steps with the new version's KoboRoot.tgz.
Unlike the script installer, this manual method always needs the reboot in
step 4 - there's no lightweight in-place update without one.
"@
Set-Content -LiteralPath (Join-Path $manualStage 'INSTRUCTIONS.txt') -Value $instructions -NoNewline

$manualZip = Join-Path $dist "KoThok-$Version-manual-install.zip"
if (Test-Path $manualZip) { Remove-Item -LiteralPath $manualZip -Force }
Compress-Archive -Path (Join-Path $manualStage '*') -DestinationPath $manualZip
Write-Host ("Manual install: {0} ({1} MB)" -f $manualZip,
    [math]::Round((Get-Item -LiteralPath $manualZip).Length / 1MB, 2))

# --- report ------------------------------------------------------------------
$size = [math]::Round((Get-Item -LiteralPath $tgz).Length / 1MB, 2)
$hash = (Get-FileHash -LiteralPath $tgz -Algorithm MD5).Hash
Write-Host ""
Write-Host "Built: $tgz"
Write-Host "Size:  $size MB"
Write-Host "MD5:   $hash"
Write-Host ""
Write-Host "Next: copy $outName to the Kobo .kobo folder over USB (rename to KoboRoot.tgz), eject, hold power 30s to reboot."
