<#
.SYNOPSIS
    Build and deploy KoThok to a Kobo e-reader over USB.
.DESCRIPTION
    Single entry point for both first install and updates.
    Auto-detects which is needed:
      - If .adds/kothok already exists on the device  -> update (copy binary)
      - If not, and libnm.so is available              -> first install (KoboRoot.tgz)
      - If not, and libnm.so is missing                -> error with instructions
.PARAMETER SkipBuild
    Skip the build step (use the existing binary).
.PARAMETER ForceFirstInstall
    Force the first-install path even if the binary already exists.
.EXAMPLE
    pwsh scripts/deploy.ps1
    pwsh scripts/deploy.ps1 -SkipBuild
    pwsh scripts/deploy.ps1 -ForceFirstInstall
#>
[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [switch]$ForceFirstInstall
)

$ErrorActionPreference = 'Stop'
$ScriptDir  = Split-Path -Parent $PSCommandPath
$ProjectDir = Split-Path -Parent $ScriptDir
$Binary     = Join-Path $ProjectDir 'target\armv7-unknown-linux-musleabihf\release\kothok'
$Libnm      = Join-Path $ScriptDir '..\package\assets\libnm.so'
$FontSrc    = Join-Path $ScriptDir '..\package\fonts'

function Step($msg)  { Write-Host "`n  $msg" -ForegroundColor White }
function Ok($msg)    { Write-Host "  [OK] $msg" -ForegroundColor Green }
function Info($msg)  { Write-Host "  $msg" -ForegroundColor DarkGray }
function Fail($msg)  {
    Write-Host ""
    Write-Host "  $msg" -ForegroundColor Red
    Write-Host ""
    exit 1
}

# Keep the device's font set in step with the binary. The app looks faces up by
# name from FONT_SPECS, so a binary that knows 23 scripts on a device carrying
# five renders the other eighteen as blank boxes - and nothing in the app
# reports that, it just draws nothing.
function Sync-Fonts($fontSrc, $addsDir) {
    if (-not (Test-Path -LiteralPath $fontSrc)) {
        Info "No fonts staged at $fontSrc - run scripts\fetch-fonts.ps1"
        Info "Non-Latin books will render as blank boxes."
        return
    }
    $faces = Get-ChildItem -LiteralPath $fontSrc -Filter '*.ttf'
    if ($faces.Count -eq 0) { return }

    $fontDir = Join-Path $addsDir 'fonts'
    New-Item -ItemType Directory -Force -Path $fontDir | Out-Null

    $copied = 0
    foreach ($f in $faces) {
        $dest = Join-Path $fontDir $f.Name
        if ((Test-Path -LiteralPath $dest) -and ((Get-Item -LiteralPath $dest).Length -eq $f.Length)) {
            continue
        }
        Copy-Item -LiteralPath $f.FullName -Destination $dest -Force
        $copied++
    }
    $total = (Get-ChildItem -LiteralPath $fontDir -Filter '*.ttf').Count
    if ($copied -gt 0) { Ok "Fonts: $total on device ($copied updated)" }
    else { Info "Fonts: $total on device (all current)" }
}

# The onboarding guide only ships inside the first-install tgz; without this
# sync an update that bumps the version would re-open the guide (the app opens
# it once per version change) with chapters from the old build. The chapter
# cache keys on the epub's mtime, so a replaced file re-parses cleanly.
function Sync-Guide($koboRoot) {
    $src = Join-Path $ScriptDir '..\samples\welcome.epub'
    if (-not (Test-Path -LiteralPath $src)) {
        Info "No guide staged at $src - run kothok-media\make-tutorial.ps1"
        return
    }
    $booksDir = Join-Path $koboRoot 'books'
    $dest = Join-Path $booksDir 'KoThok - Getting Started.epub'
    if (Test-Path -LiteralPath $dest) {
        $srcHash = (Get-FileHash -LiteralPath $src -Algorithm MD5).Hash
        $dstHash = (Get-FileHash -LiteralPath $dest -Algorithm MD5).Hash
        if ($srcHash -eq $dstHash) { Info "Guide: current"; return }
    }
    New-Item -ItemType Directory -Force -Path $booksDir | Out-Null
    Copy-Item -LiteralPath $src -Destination $dest -Force
    Ok "Guide: updated"
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
Write-Host "  KoThok USB Deploy" -ForegroundColor Cyan
Write-Host "  =================" -ForegroundColor Cyan

# --- 0. Check prerequisites --------------------------------------------------
Step "Checking prerequisites..."

$missing = @()

# Presence is checked with Get-Command, not by running the tool. Under
# `$ErrorActionPreference = 'Stop'` a native command that writes *anything* to
# stderr raises a NativeCommandError, which the catch then swallows as "not
# installed" -- and `cross --version` always prints a host-target warning. That
# reported a perfectly good toolchain as missing.
if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
    $missing += "Rust (install from https://rustup.rs)"
}

if (-not (Get-Command cross -ErrorAction SilentlyContinue)) {
    $missing += "cross (run: cargo install cross)"
}

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    $missing += "Docker (install from https://docker.com)"
} else {
    # The daemon has to actually be reachable, so this one does run -- with
    # stderr merged into stdout so a warning cannot be mistaken for a failure.
    $dockerRunning = $false
    try {
        & docker info 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) { $dockerRunning = $true }
    } catch {}
    if (-not $dockerRunning) {
        $missing += "Docker daemon not running (start Docker Desktop)"
    }
}

if ($missing.Count -gt 0) {
    Write-Host ""
    foreach ($m in $missing) { Write-Host "  [X] $m" -ForegroundColor Red }
    Write-Host ""
    Write-Host "  Install the missing tools, then re-run this script." -ForegroundColor Yellow
    Write-Host ""
    exit 1
}

Ok "Rust, cross, Docker all available"

# --- 1. Build ----------------------------------------------------------------
if (-not $SkipBuild) {
    Step "Building release binary..."

    $buildJob = Start-Job -ScriptBlock {
        param($pd)
        Set-Location $pd
        $output = & cross build --target armv7-unknown-linux-musleabihf --release -p kothok-app 2>&1
        foreach ($line in $output) { Write-Output $line }
        Write-Output "EXITCODE:$LASTEXITCODE"
    } -ArgumentList $ProjectDir

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $maxSec = 120
    while ($job = Get-Job -Id $buildJob.Id -ErrorAction SilentlyContinue) {
        if ($job.State -ne 'Running') { break }
        $elapsedSec = [int]$sw.Elapsed.TotalSeconds
        $pct = [Math]::Min(95, [int]($elapsedSec * 100.0 / $maxSec))
        Write-Progress -Activity "Building KoThok" `
            -Status "Compiling... (${elapsedSec}s)" `
            -PercentComplete $pct
        Start-Sleep -Milliseconds 500
    }
    $sw.Stop()
    Write-Progress -Activity "Building KoThok" -Completed

    $jobOutput = Receive-Job $buildJob
    Remove-Job $buildJob -Force

    $buildExit = 1
    foreach ($line in $jobOutput) {
        if ($line -is [string] -and $line -match '^EXITCODE:(.+)$') {
            $buildExit = [int]$Matches[1]
        }
    }

    if ($buildExit -ne 0) {
        $jobOutput | Where-Object { $_ -is [string] -and $_ -notmatch '^EXITCODE:' } |
            ForEach-Object { Write-Host "  $_" -ForegroundColor DarkGray }
        Fail "Build failed. Fix errors above and re-run."
    }

    Ok "Build complete ($([int]$sw.Elapsed.TotalSeconds)s)"
} else {
    Info "Skipping build (-SkipBuild)"
}

if (-not (Test-Path -LiteralPath $Binary)) {
    Fail "Binary not found at: $Binary  (build first, remove -SkipBuild)"
}

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

# --- 3. Decide: first install or update --------------------------------------
$isFirstInstall = $ForceFirstInstall -or (-not (Test-Path -LiteralPath $binaryOnDevice))

if ($isFirstInstall) {
    if (-not (Test-Path -LiteralPath $Libnm)) {
        Fail "First install needs libnm.so at: $Libnm"
    }

    Step "First install - building KoboRoot.tgz..."

    $toml = Get-Content (Join-Path $ProjectDir 'Cargo.toml') -Raw
    $verMatch = [regex]::Match($toml, 'version\s*=\s*"(.+?)"')
    if ($verMatch.Success) { $version = $verMatch.Groups[1].Value }
    else { Fail "Could not read version from Cargo.toml" }

    $main = Get-Content (Join-Path $ProjectDir 'src\main.rs') -Raw
    $tagMatch = [regex]::Match($main, 'BUILD_TAG:\s*\S+\s*=\s*"(.+?)"')
    if ($tagMatch.Success) { $buildTag = $tagMatch.Groups[1].Value }
    else { $buildTag = 'unknown' }

    $runSh = Join-Path $ProjectDir 'run.sh'
    $nmCfg = Join-Path $ScriptDir '..\package\nm\config'

    $stage = Join-Path $ScriptDir '..\package\.stage'
    $dist  = Join-Path $ScriptDir '..\package\dist'
    if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
    New-Item -ItemType Directory -Force -Path $dist | Out-Null

    foreach ($d in 'mnt\onboard\.adds\nm', 'usr\local\Kobo\imageformats') {
        New-Item -ItemType Directory -Force -Path (Join-Path $stage $d) | Out-Null
    }

    Copy-Item -LiteralPath $Binary -Destination (Join-Path $stage 'mnt\onboard\.adds\kothok')
    Copy-Item -LiteralPath $runSh  -Destination (Join-Path $stage 'mnt\onboard\.adds\run.sh')
    Copy-Item -LiteralPath $nmCfg  -Destination (Join-Path $stage 'mnt\onboard\.adds\nm\config')
    Copy-Item -LiteralPath $Libnm  -Destination (Join-Path $stage 'usr\local\Kobo\imageformats\libnm.so')

    if (Test-Path -LiteralPath $FontSrc) {
        $stagedFonts = Join-Path $stage 'mnt\onboard\.adds\fonts'
        New-Item -ItemType Directory -Force -Path $stagedFonts | Out-Null
        $faces = Get-ChildItem -LiteralPath $FontSrc -Filter '*.ttf'
        foreach ($f in $faces) { Copy-Item -LiteralPath $f.FullName -Destination $stagedFonts }
        Info "Bundling $($faces.Count) font face(s)"
    } else {
        Info "No fonts staged - run scripts\fetch-fonts.ps1 for non-Latin scripts"
    }

    $verFile = Join-Path $stage 'mnt\onboard\.adds\kothok-version'
    "KoThok $version ($buildTag)" | Set-Content -LiteralPath $verFile

    $sampleSrc = Join-Path $ScriptDir '..\samples'
    if (Test-Path -LiteralPath $sampleSrc) {
        $sampleDir = Join-Path $stage 'mnt\onboard\books'
        New-Item -ItemType Directory -Force -Path $sampleDir | Out-Null
        foreach ($epub in (Get-ChildItem -LiteralPath $sampleSrc -Filter '*.epub')) {
            # The guide is looked up by name (GUIDE_PATH), so it must land as
            # "KoThok - Getting Started.epub", same as make-release.ps1 stages
            # it. Under its build name the app never finds it and onboarding
            # silently never runs - plus the shelf shows a "welcome" duplicate.
            if ($epub.Name -eq 'welcome.epub') {
                Copy-Item -LiteralPath $epub.FullName `
                    -Destination (Join-Path $sampleDir 'KoThok - Getting Started.epub')
                Info "Bundling onboarding guide"
            } else {
                Copy-Item -LiteralPath $epub.FullName -Destination $sampleDir
            }
        }
        Info "Bundling sample book(s) so a fresh device has something to read"
    }

    $outName = "KoThok-$version.KoboRoot.tgz"
    $tgz     = Join-Path $dist $outName

    Info "Packing $outName ..."
    Push-Location $stage
    try {
        & tar czf $tgz .
        if ($LASTEXITCODE -ne 0) { Fail "tar failed with exit code $LASTEXITCODE" }
    } finally { Pop-Location }

    if (-not (Test-Path -LiteralPath $tgz)) { Fail "tar did not produce $tgz" }

    $koboDir = Join-Path $koboRoot '.kobo'
    Copy-Item -LiteralPath $tgz -Destination (Join-Path $koboDir 'KoboRoot.tgz') -Force
    $hash = (Get-FileHash -LiteralPath $tgz -Algorithm MD5).Hash

    Write-Host ""
    Write-Host "  ============================" -ForegroundColor Green
    Write-Host "  FIRST INSTALL COMPLETE" -ForegroundColor Green
    Write-Host "  ============================" -ForegroundColor Green
    Info "Copied: $outName -> .kobo/KoboRoot.tgz"
    Info "MD5:    $hash"
    Write-Host ""
    Write-Host "  DONE! Follow these steps:" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "  1. Eject the Kobo (system tray -> Safely Remove -> KOBOeReader)" -ForegroundColor White
    Write-Host "  2. Unplug USB cable" -ForegroundColor White
    Write-Host "  3. Reboot the Kobo (power it off and back on)" -ForegroundColor White
    Write-Host "  4. Watch for 'Updating...' screen (~30s) - this installs NickelMenu" -ForegroundColor White
    Write-Host "     and KoThok, then it restarts itself" -ForegroundColor White
    Write-Host "  5. Wait for nickel home screen to appear" -ForegroundColor White
    Write-Host "  6. Tap the hamburger menu (bottom-right) -> tap 'KoThok'" -ForegroundColor White
    Write-Host ""
}
else {
    Step "Update - copying binary..."
    $hostHash = (Get-FileHash -LiteralPath $Binary -Algorithm MD5).Hash.ToLower()

    Copy-Item -LiteralPath $Binary -Destination $binaryOnDevice -Force
    $devHash = (Get-FileHash -LiteralPath $binaryOnDevice -Algorithm MD5).Hash.ToLower()

    if ($hostHash -ne $devHash) {
        Fail "MD5 mismatch! host=$hostHash dev=$devHash"
    }

    Sync-Fonts $FontSrc $addsDir

    Sync-Guide $koboRoot

    # The build stamp the app will show for this binary. Read back from the
    # device copy, so it describes the file that was actually written.
    #
    # The MD5 check above is weaker than it looks: it re-reads through the same
    # Windows file cache the copy just went into, so it confirms bytes that may
    # still only exist in RAM. Comparing this pair against the settings panel is
    # what proves the binary reached flash and survived the reboot run.sh does
    # on exit.
    $devFile  = Get-Item -LiteralPath $binaryOnDevice
    $epoch    = [int64](New-TimeSpan -Start ([datetime]'1970-01-01Z').ToUniversalTime() `
                                     -End $devFile.LastWriteTimeUtc).TotalSeconds
    # kc: suffix must match the binary's own BUILD stamp, which build.rs bakes
    # from the kobo-core version resolved in Cargo.lock (not the local git rev --
    # the path patch is gone; kobo-core comes from crates.io). Same source, or
    # the host print and the device BUILD line disagree.
    $lockFile = Join-Path $PSScriptRoot "../Cargo.lock"
    $kcVer    = "?"
    if (Test-Path $lockFile) {
        $want = $false
        foreach ($ln in Get-Content -LiteralPath $lockFile) {
            $t = $ln.Trim()
            if ($t -eq "[[package]]") { $want = $false }
            elseif ($t.StartsWith('name = "kobo-core"')) { $want = $true }
            elseif ($want -and $t.StartsWith("version =")) {
                $kcVer = ($t -replace '^version =\s*"([^"]*)".*', '$1')
                break
            }
        }
    }
    $stamp    = "$([int64]($devFile.Length / 1024))k/$($epoch % 1000000) kc:$kcVer"

    Write-Host ""
    Write-Host "  ============================" -ForegroundColor Green
    Write-Host "  UPDATE COMPLETE" -ForegroundColor Green
    Write-Host "  ============================" -ForegroundColor Green
    Info "MD5:   $hostHash"
    Write-Host "  BUILD: $stamp" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "  DONE! Follow these steps:" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "  1. Eject the Kobo (system tray -> Safely Remove -> KOBOeReader)" -ForegroundColor White
    Write-Host "     Do NOT just unplug -- an unflushed copy is lost on the next reboot," -ForegroundColor DarkGray
    Write-Host "     and run.sh reboots the device every time you exit the reader." -ForegroundColor DarkGray
    Write-Host "  2. Unplug USB cable" -ForegroundColor White
    Write-Host "  3. Tap the hamburger menu (bottom-right) -> tap 'KoThok'" -ForegroundColor White
    Write-Host "  4. Library -> gear -> check the build line reads:" -ForegroundColor White
    Write-Host "        build $stamp" -ForegroundColor Cyan
    Write-Host "     If it does not, the copy did not survive. Re-run with a proper eject." -ForegroundColor DarkGray
    Write-Host ""
}
