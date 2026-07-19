<#
.SYNOPSIS
    Download the Noto font set KoThok ships to the device.
.DESCRIPTION
    Reading must never need a network connection, so every script's face is
    installed up front rather than fetched on demand. This script pulls them
    once at packaging time; the result is bundled into the release and copied
    to `.adds/fonts` by the installer.

    The file names here must match `FONT_SPECS` in
    `kothok/kobo-core/src/device/fonts.rs` - the app looks each face up by name.
.PARAMETER OutDir
    Where the fonts land. Defaults to the gitignored staging folder.
.PARAMETER SkipCjk
    Omit Chinese, Japanese and Korean. They are ~22MB of the ~24MB total, so
    this produces a set covering 20 of 23 scripts for under 2MB.
.EXAMPLE
    .\fetch-fonts.ps1
.EXAMPLE
    .\fetch-fonts.ps1 -SkipCjk
#>
[CmdletBinding()]
param(
    [string]$OutDir = 'D:\Programming\BitOps\EReader\kothok\package\fonts',
    [switch]$SkipCjk
)

$ErrorActionPreference = 'Stop'

function Step($msg) { Write-Host "`n  $msg" -ForegroundColor White }
function Ok($msg)   { Write-Host "  [OK] $msg" -ForegroundColor Green }
function Info($msg) { Write-Host "  $msg" -ForegroundColor DarkGray }
function Warn($msg) { Write-Host "  [!] $msg" -ForegroundColor Yellow }

$NOTO = 'https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts'
$CJK  = 'https://github.com/notofonts/noto-cjk/raw/main/Sans/SubsetOTF'

# Target name -> source URL. Target names are what FONT_SPECS looks for.
$FONTS = [ordered]@{
    # Base face: Latin, Greek and Cyrillic all live here.
    'NotoSans.ttf'           = "$NOTO/NotoSans/hinted/ttf/NotoSans-Regular.ttf"
    'NotoSansBengali.ttf'    = "$NOTO/NotoSansBengali/hinted/ttf/NotoSansBengali-Regular.ttf"
    'NotoSansDevanagari.ttf' = "$NOTO/NotoSansDevanagari/hinted/ttf/NotoSansDevanagari-Regular.ttf"
    'NotoSansArabic.ttf'     = "$NOTO/NotoSansArabic/hinted/ttf/NotoSansArabic-Regular.ttf"
    'NotoSansHebrew.ttf'     = "$NOTO/NotoSansHebrew/hinted/ttf/NotoSansHebrew-Regular.ttf"
    'NotoSansGeorgian.ttf'   = "$NOTO/NotoSansGeorgian/hinted/ttf/NotoSansGeorgian-Regular.ttf"
    'NotoSansArmenian.ttf'   = "$NOTO/NotoSansArmenian/hinted/ttf/NotoSansArmenian-Regular.ttf"
    'NotoSansEthiopic.ttf'   = "$NOTO/NotoSansEthiopic/hinted/ttf/NotoSansEthiopic-Regular.ttf"
    'NotoSansGujarati.ttf'   = "$NOTO/NotoSansGujarati/hinted/ttf/NotoSansGujarati-Regular.ttf"
    'NotoSansGurmukhi.ttf'   = "$NOTO/NotoSansGurmukhi/hinted/ttf/NotoSansGurmukhi-Regular.ttf"
    'NotoSansTamil.ttf'      = "$NOTO/NotoSansTamil/hinted/ttf/NotoSansTamil-Regular.ttf"
    'NotoSansTelugu.ttf'     = "$NOTO/NotoSansTelugu/hinted/ttf/NotoSansTelugu-Regular.ttf"
    'NotoSansKannada.ttf'    = "$NOTO/NotoSansKannada/hinted/ttf/NotoSansKannada-Regular.ttf"
    'NotoSansMalayalam.ttf'  = "$NOTO/NotoSansMalayalam/hinted/ttf/NotoSansMalayalam-Regular.ttf"
    'NotoSansSinhala.ttf'    = "$NOTO/NotoSansSinhala/hinted/ttf/NotoSansSinhala-Regular.ttf"
    'NotoSansThai.ttf'       = "$NOTO/NotoSansThai/hinted/ttf/NotoSansThai-Regular.ttf"
    'NotoSansLao.ttf'        = "$NOTO/NotoSansLao/hinted/ttf/NotoSansLao-Regular.ttf"
    'NotoSansKhmer.ttf'      = "$NOTO/NotoSansKhmer/hinted/ttf/NotoSansKhmer-Regular.ttf"
    'NotoSansMyanmar.ttf'    = "$NOTO/NotoSansMyanmar/hinted/ttf/NotoSansMyanmar-Regular.ttf"
}

# CJK ships as CFF-outline OTF; noto-cjk publishes no static TTF. The .ttf
# target names keep FONT_SPECS uniform - the loader parses by content, not
# extension.
$CJK_FONTS = [ordered]@{
    'NotoSansJP.ttf' = "$CJK/JP/NotoSansJP-Regular.otf"
    'NotoSansSC.ttf' = "$CJK/SC/NotoSansSC-Regular.otf"
    'NotoSansKR.ttf' = "$CJK/KR/NotoSansKR-Regular.otf"
}

if (-not $SkipCjk) {
    foreach ($k in $CJK_FONTS.Keys) { $FONTS[$k] = $CJK_FONTS[$k] }
}

Write-Host ""
Write-Host "  KoThok Font Fetch" -ForegroundColor Cyan
Write-Host "  =================" -ForegroundColor Cyan

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
Step "Downloading $($FONTS.Count) font(s) to $OutDir"

$failed = @()
foreach ($name in $FONTS.Keys) {
    $dest = Join-Path $OutDir $name
    if (Test-Path $dest) {
        Info "$name (already present, skipping)"
        continue
    }
    try {
        Invoke-WebRequest -Uri $FONTS[$name] -OutFile $dest -UseBasicParsing -TimeoutSec 180
        $kb = [math]::Round((Get-Item $dest).Length / 1KB)
        Ok "$name  ($kb KB)"
    } catch {
        $failed += $name
        Warn "$name FAILED: $($_.Exception.Message)"
        if (Test-Path $dest) { Remove-Item $dest -Force }
    }
}

# --- verify -----------------------------------------------------------------
Step "Verifying"
$all = Get-ChildItem $OutDir -Filter '*.ttf'
$total = ($all | Measure-Object Length -Sum).Sum

foreach ($f in $all) {
    # sfnt magic: 0x00010000 (TrueType) or "OTTO" (CFF outlines).
    $head = [byte[]](Get-Content -LiteralPath $f.FullName -AsByteStream -TotalCount 4)
    $isTtf = $head[0] -eq 0x00 -and $head[1] -eq 0x01 -and $head[2] -eq 0x00 -and $head[3] -eq 0x00
    $isOtf = $head[0] -eq 0x4F -and $head[1] -eq 0x54 -and $head[2] -eq 0x54 -and $head[3] -eq 0x4F
    if (-not ($isTtf -or $isOtf)) {
        Warn "$($f.Name): not a valid sfnt font (bad magic) - likely an error page"
    }
}

Write-Host ""
Write-Host "  ============================" -ForegroundColor Green
Write-Host "  $($all.Count) font(s), $([math]::Round($total/1MB,1)) MB total" -ForegroundColor Green
Write-Host "  ============================" -ForegroundColor Green
if ($failed.Count -gt 0) {
    Warn "$($failed.Count) failed: $($failed -join ', ')"
    exit 1
}
Write-Host ""
