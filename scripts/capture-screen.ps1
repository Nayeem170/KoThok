<#
.SYNOPSIS
    Grab the Kobo framebuffer and save it as a PNG for the marketing site.
.DESCRIPTION
    Dumps /dev/fb0 to the onboard .adds/shots folder over telnet, then converts
    the raw RGB565 buffer to PNG once the device is plugged in over USB.

    Capture and collect are separate steps because the device cannot serve
    telnet and USB mass storage at the same time. Capture every screen you want
    in one WiFi session, then plug in the cable once and collect them all.

    Screen geometry is read from sysfs rather than hardcoded, so this keeps
    working if the panel or rotation changes.
.PARAMETER Name
    Screen name, used as the output filename (e.g. "reading", "library").
.PARAMETER Ip
    Device IP on the local network.
.PARAMETER Collect
    Skip capture. Convert every .raw already sitting on the mounted Kobo.
.PARAMETER OutDir
    Where the PNGs land. Defaults to the kothok-site public screens folder.
.EXAMPLE
    # On the device: navigate to the reading view, then:
    .\capture-screen.ps1 -Name reading -Ip 192.168.0.93
.EXAMPLE
    # After capturing every screen, plug in USB and:
    .\capture-screen.ps1 -Collect
#>
[CmdletBinding(DefaultParameterSetName = 'Capture')]
param(
    [Parameter(ParameterSetName = 'Capture', Mandatory = $true, Position = 0)]
    [string]$Name,

    [Parameter(ParameterSetName = 'Capture')]
    [string]$Ip = '192.168.0.93',

    [Parameter(ParameterSetName = 'Collect', Mandatory = $true)]
    [switch]$Collect,

    [string]$OutDir = 'D:\Programming\BitOps\kothok-site\public\screens'
)

$ErrorActionPreference = 'Stop'

$DeviceShotDir = '/mnt/onboard/.adds/shots'

function Step($msg) { Write-Host "`n  $msg" -ForegroundColor White }
function Ok($msg)   { Write-Host "  [OK] $msg" -ForegroundColor Green }
function Info($msg) { Write-Host "  $msg" -ForegroundColor DarkGray }
function Warn($msg) { Write-Host "  [!] $msg" -ForegroundColor Yellow }
function Fail($msg) { Write-Host ""; Write-Host "  $msg" -ForegroundColor Red; Write-Host ""; exit 1 }

# --- telnet plumbing ---------------------------------------------------------
# Paced line-at-a-time writes with TERM=dumb; the device's telnetd drops input
# that arrives faster than it echoes.

function Connect-Kobo([string]$ip) {
    $client = New-Object System.Net.Sockets.TcpClient
    try {
        $client.Connect($ip, 23)
    } catch {
        Fail @"
Could not reach the Kobo at ${ip}:23.

  - Unplug the USB cable (mass storage mode stops the app and telnetd).
  - Connect the device to WiFi.
  - Confirm the IP in the device's network settings, pass it with -Ip.
"@
    }
    $stream = $client.GetStream()
    $stream.ReadTimeout = 4000
    Start-Sleep -Milliseconds 1500
    return @{ Client = $client; Stream = $stream }
}

function Send-Line($session, [string]$text) {
    $bytes = [Text.Encoding]::ASCII.GetBytes($text + "`n")
    $session.Stream.Write($bytes, 0, $bytes.Length)
    $session.Stream.Flush()
    Start-Sleep -Milliseconds 600
}

function Read-Drain($session) {
    $buf = New-Object byte[] 16384
    $sb = New-Object Text.StringBuilder
    try {
        while ($true) {
            $n = $session.Stream.Read($buf, 0, $buf.Length)
            if ($n -le 0) { break }
            [void]$sb.Append([Text.Encoding]::ASCII.GetString($buf, 0, $n))
        }
    } catch { }
    return $sb.ToString()
}

# --- RGB565 -> PNG -----------------------------------------------------------
# Inverse of kobo_core::device::fb::dump_ppm: little-endian u16, RRRRRGGGGGGBBBBB,
# with the low bits replicated into the gap so white stays fully white.

function Convert-Rgb565ToPng([string]$rawPath, [string]$pngPath, [int]$w, [int]$h) {
    Add-Type -AssemblyName System.Drawing

    $raw = [System.IO.File]::ReadAllBytes($rawPath)
    $expected = $w * $h * 2
    if ($raw.Length -lt $expected) {
        Fail "$(Split-Path -Leaf $rawPath): expected $expected bytes for ${w}x${h}, got $($raw.Length)."
    }

    $bmp = New-Object System.Drawing.Bitmap($w, $h, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
    $rect = New-Object System.Drawing.Rectangle(0, 0, $w, $h)
    $data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::WriteOnly, $bmp.PixelFormat)
    try {
        $stride = $data.Stride
        $out = New-Object byte[] ($stride * $h)
        for ($y = 0; $y -lt $h; $y++) {
            $rowOut = $y * $stride
            $rowIn = $y * $w * 2
            for ($x = 0; $x -lt $w; $x++) {
                $off = $rowIn + $x * 2
                $v = [int]$raw[$off] -bor ([int]$raw[$off + 1] -shl 8)
                $r = ($v -shr 11) -band 0x1f
                $g = ($v -shr 5) -band 0x3f
                $b = $v -band 0x1f
                $p = $rowOut + $x * 3
                # GDI+ 24bpp is BGR order.
                $out[$p]     = [byte](($b -shl 3) -bor ($b -shr 2))
                $out[$p + 1] = [byte](($g -shl 2) -bor ($g -shr 4))
                $out[$p + 2] = [byte](($r -shl 3) -bor ($r -shr 2))
            }
        }
        [System.Runtime.InteropServices.Marshal]::Copy($out, 0, $data.Scan0, $out.Length)
    } finally {
        $bmp.UnlockBits($data)
    }

    $bmp.Save($pngPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

function Find-Kobo {
    $d = Get-PSDrive -PSProvider FileSystem | Where-Object {
        Test-Path (Join-Path $_.Root '.adds')
    } | Select-Object -First 1
    if ($d) { return $d.Root }
    return $null
}

Write-Host ""
Write-Host "  KoThok Screen Capture" -ForegroundColor Cyan
Write-Host "  =====================" -ForegroundColor Cyan

# --- collect mode ------------------------------------------------------------
if ($Collect) {
    Step "Looking for Kobo over USB..."
    $koboRoot = Find-Kobo
    if (-not $koboRoot) { Fail "Kobo not mounted. Plug in the USB cable and unlock the device." }
    Ok "Found Kobo at $koboRoot"

    $shotDir = Join-Path $koboRoot '.adds\shots'
    if (-not (Test-Path -LiteralPath $shotDir)) {
        Fail "No .adds\shots folder. Capture some screens over WiFi first."
    }

    $raws = Get-ChildItem -LiteralPath $shotDir -Filter '*.raw'
    if ($raws.Count -eq 0) { Fail "No .raw captures in $shotDir." }

    New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

    Step "Converting $($raws.Count) capture(s)..."
    foreach ($raw in $raws) {
        $metaPath = [IO.Path]::ChangeExtension($raw.FullName, '.txt')
        if (-not (Test-Path -LiteralPath $metaPath)) {
            Warn "$($raw.Name): no sidecar geometry file, skipping."
            continue
        }
        $meta = Get-Content -LiteralPath $metaPath -Raw
        if ($meta -notmatch '(\d+)\s*,\s*(\d+)') {
            Warn "$($raw.Name): could not parse geometry from sidecar, skipping."
            continue
        }
        $w = [int]$Matches[1]
        $h = [int]$Matches[2]

        $png = Join-Path $OutDir ([IO.Path]::ChangeExtension($raw.Name, '.png'))
        Convert-Rgb565ToPng $raw.FullName $png $w $h
        Ok "$($raw.BaseName).png  (${w}x${h})"
    }

    Write-Host ""
    Write-Host "  Wrote PNGs to $OutDir" -ForegroundColor Green
    Info "Encode to WebP before committing - these are large as PNG."
    Write-Host ""
    exit 0
}

# --- capture mode ------------------------------------------------------------
Step "Connecting to $Ip..."
$session = Connect-Kobo $Ip
[void](Read-Drain $session)
Send-Line $session ''
Send-Line $session 'export TERM=dumb'
[void](Read-Drain $session)
Ok "Connected"

Step "Reading framebuffer geometry..."
Send-Line $session 'echo ===GEO===; cat /sys/class/graphics/fb0/virtual_size; cat /sys/class/graphics/fb0/bits_per_pixel; echo ===END==='
Start-Sleep -Milliseconds 1200
$geo = Read-Drain $session

if ($geo -notmatch '(?m)^\s*(\d+)\s*,\s*(\d+)\s*$') {
    Fail "Could not read fb0 virtual_size. Raw response:`n$geo"
}
$w = [int]$Matches[1]
$h = [int]$Matches[2]

$bpp = 16
if ($geo -match '(?m)^\s*(8|16|24|32)\s*$') { $bpp = [int]$Matches[1] }
if ($bpp -ne 16) {
    Fail "fb0 reports ${bpp}bpp. This script only converts RGB565 (16bpp)."
}

$bytes = $w * $h * 2
Ok "fb0 is ${w}x${h} @ ${bpp}bpp ($([math]::Round($bytes / 1MB, 1)) MB per frame)"

Step "Capturing '$Name'..."
Send-Line $session "mkdir -p $DeviceShotDir"
Send-Line $session "dd if=/dev/fb0 of=$DeviceShotDir/$Name.raw bs=$bytes count=1 2>/dev/null"
Send-Line $session "echo $w,$h > $DeviceShotDir/$Name.txt"
# Onboard writes are lost on reboot without an explicit flush.
Send-Line $session 'sync'
Start-Sleep -Milliseconds 1500

Send-Line $session "echo ===SIZE===; wc -c < $DeviceShotDir/$Name.raw; echo ===END==="
Start-Sleep -Milliseconds 1200
$sizeOut = Read-Drain $session

$session.Client.Close()

if ($sizeOut -match '(?m)^\s*(\d+)\s*$') {
    $got = [int]$Matches[1]
    if ($got -ne $bytes) {
        Fail "Short read: got $got bytes, expected $bytes."
    }
    Ok "Captured $DeviceShotDir/$Name.raw ($got bytes)"
} else {
    Warn "Could not verify capture size. Response:`n$sizeOut"
}

Write-Host ""
Write-Host "  Next:" -ForegroundColor Yellow
Info "Capture the remaining screens, then plug in USB and run:"
Info "  .\capture-screen.ps1 -Collect"
Write-Host ""
