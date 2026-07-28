<#
.SYNOPSIS
  Builds the KoThok onboarding guide epub (samples/welcome.epub).

.DESCRIPTION
  Reads the top section of CHANGELOG.md and renders it as the "What's New"
  chapter (ch00). Stages all tutorial XHTML, CSS, cover image, OPF and NCX
  into a valid EPUB 2 archive.

  The mimetype entry is stored uncompressed (EPUB spec requirement); every
  other entry is deflate-compressed.

.USAGE
  .\make-tutorial.ps1
#>

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$ScriptDir = $PSScriptRoot
$TutorialDir = Join-Path $ScriptDir '..\tutorial'
$SamplesDir = Join-Path $ScriptDir '..\samples'
$Changelog  = Join-Path $ScriptDir '..\..\CHANGELOG.md'

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
Add-Type -AssemblyName System.Drawing

# ---------------------------------------------------------------------------
# Version
# ---------------------------------------------------------------------------
function Read-CargoVersion {
    $toml = Get-Content (Join-Path $ScriptDir '..\Cargo.toml') -Raw
    if ($toml -match '(?m)^version\s*=\s*"([^"]+)"') { return $matches[1] }
    throw 'Could not read version from Cargo.toml'
}

$Version = Read-CargoVersion
$BuildTag = "v$Version"
Write-Host "Building onboarding guide for $BuildTag"

# ---------------------------------------------------------------------------
# Chapter manifest (order = spine order)
# ---------------------------------------------------------------------------
# ch00 is generated; ch01-ch08 are static; ch09 is the appendix.
$Chapters = @(
    @{ Id = 'ch00'; File = 'ch00-whats-new.xhtml';        Title = "What's New in $BuildTag"; Generated = $true }
    @{ Id = 'ch01'; File = 'ch01-welcome.xhtml';          Title = 'Welcome';                 Generated = $false }
    @{ Id = 'ch02'; File = 'ch02-turning-pages.xhtml';    Title = 'Turning Pages';           Generated = $false }
    @{ Id = 'ch03'; File = 'ch03-listening.xhtml';        Title = 'Listening';               Generated = $false }
    @{ Id = 'ch04'; File = 'ch04-settings.xhtml';         Title = 'Settings';                Generated = $false }
    @{ Id = 'ch05'; File = 'ch05-finding-your-place.xhtml'; Title = 'Finding Your Place';    Generated = $false }
    @{ Id = 'ch06'; File = 'ch06-reading-vs-audio.xhtml'; Title = 'Reading vs Audio Mode';   Generated = $false }
    @{ Id = 'ch07'; File = 'ch07-zoom-and-links.xhtml';   Title = 'Zoom and Links';          Generated = $false }
    @{ Id = 'ch08'; File = 'ch08-library-and-exit.xhtml'; Title = 'Your Library and Leaving'; Generated = $false }
    @{ Id = 'ch09'; File = 'ch09-earlier-updates.xhtml';  Title = 'Earlier Updates';         Generated = $false }
)

# ---------------------------------------------------------------------------
# Generate ch00 (What's New) from CHANGELOG.md
# ---------------------------------------------------------------------------
function ConvertTo-HtmlSafe([string]$s) {
    $s = $s -replace '&', '&amp;'
    $s = $s -replace '<', '&lt;'
    $s = $s -replace '>', '&gt;'
    return $s
}

function Build-WhatsNew {
    $lines = Get-Content $Changelog
    $body = [System.Text.StringBuilder]::new()
    [void]$body.AppendLine('<h2>' + (ConvertTo-HtmlSafe "What's New in $BuildTag") + '</h2>')

    $inSection = $false
    foreach ($line in $lines) {
        if ($line -match '^##\s*\[') {
            if ($inSection) { break }
            $inSection = $true
            continue
        }
        if (-not $inSection) { continue }
        $trimmed = $line.Trim()
        if ($trimmed -eq '') { continue }
        if ($trimmed -match '^###\s*(.+)') {
            [void]$body.AppendLine('<h3>' + (ConvertTo-HtmlSafe $matches[1].Trim()) + '</h3>')
        } elseif ($trimmed -match '^-\s*(.+)') {
            [void]$body.AppendLine('<p>' + (ConvertTo-HtmlSafe $matches[1].Trim()) + '</p>')
        }
    }

    if (-not $inSection) {
        throw "No version section found in $Changelog"
    }

    return @"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.1//EN" "http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd">
<html xmlns="http://www.w3.org/1999/xhtml">
<head>
  <title>What's New</title>
  <link rel="stylesheet" type="text/css" href="style.css"/>
</head>
<body>
$($body.ToString().TrimEnd())
</body>
</html>
"@
}

# ---------------------------------------------------------------------------
# Generate content.opf
# ---------------------------------------------------------------------------
function Build-Opf {
    $manifestItems = [System.Text.StringBuilder]::new()
    $spineItems = [System.Text.StringBuilder]::new()
    $playOrder = 1
    foreach ($ch in $Chapters) {
        [void]$manifestItems.AppendLine("    <item id=`"$($ch.Id)`" href=`"$($ch.File)`" media-type=`"application/xhtml+xml`"/>")
        [void]$spineItems.AppendLine("    <itemref idref=`"$($ch.Id)`"/>")
        $playOrder++
    }

    return @"
<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="BookId" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:opf="http://www.idpf.org/2007/opf">
    <dc:title>KoThok - Getting Started</dc:title>
    <dc:creator>Nayeem Bin Ahsan</dc:creator>
    <dc:language>en</dc:language>
    <dc:identifier id="BookId">urn:uuid:kothok-getting-started-$Version</dc:identifier>
    <meta name="cover" content="cover-image"/>
  </metadata>
  <manifest>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
    <item id="css" href="style.css" media-type="text/css"/>
    <item id="cover-image" href="cover.png" media-type="image/png"/>
$($manifestItems.ToString().TrimEnd())
  </manifest>
  <spine toc="ncx">
$($spineItems.ToString().TrimEnd())
  </spine>
</package>
"@
}

# ---------------------------------------------------------------------------
# Generate toc.ncx
# ---------------------------------------------------------------------------
function Build-Ncx {
    $navPoints = [System.Text.StringBuilder]::new()
    $playOrder = 1
    foreach ($ch in $Chapters) {
        [void]$navPoints.AppendLine(@"
    <navPoint id="$($ch.Id)" playOrder="$playOrder">
      <navLabel><text>$((ConvertTo-HtmlSafe $ch.Title))</text></navLabel>
      <content src="$($ch.File)"/>
    </navPoint>
"@)
        $playOrder++
    }

    return @"
<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head>
    <meta name="dtb:uid" content="urn:uuid:kothok-getting-started-$Version"/>
  </head>
  <docTitle><text>KoThok - Getting Started</text></docTitle>
  <navMap>
$($navPoints.ToString().TrimEnd())
  </navMap>
</ncx>
"@
}

# ---------------------------------------------------------------------------
# Generate cover image
# ---------------------------------------------------------------------------
function New-CoverImage([string]$OutPath) {
    $w = 400
    $h = 600
    $bmp = New-Object System.Drawing.Bitmap($w, $h)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAlias
    $g.Clear([System.Drawing.Color]::White)

    $redBar = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(170, 30, 30))
    $g.FillRectangle($redBar, 0, 0, $w, 12)

    $fmt = New-Object System.Drawing.StringFormat
    $fmt.Alignment = [System.Drawing.StringAlignment]::Center

    $titleFont = New-Object System.Drawing.Font('Arial', 48, [System.Drawing.FontStyle]::Bold)
    $g.DrawString('KoThok', $titleFont, [System.Drawing.Brushes]::Black,
        (New-Object System.Drawing.RectangleF(0, 220, $w, 70)), $fmt)

    $subFont = New-Object System.Drawing.Font('Arial', 26)
    $grayBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(90, 90, 90))
    $g.DrawString('Getting Started', $subFont, $grayBrush,
        (New-Object System.Drawing.RectangleF(0, 300, $w, 40)), $fmt)

    $verFont = New-Object System.Drawing.Font('Arial', 16)
    $g.DrawString($BuildTag, $verFont, $grayBrush,
        (New-Object System.Drawing.RectangleF(0, 550, $w, 30)), $fmt)

    $g.Dispose()
    $bmp.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

# ---------------------------------------------------------------------------
# Assemble EPUB
# ---------------------------------------------------------------------------
$stage = Join-Path $ScriptDir '.tutorial-stage'
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
$oebps = Join-Path $stage 'OEBPS'
$metaInf = Join-Path $stage 'META-INF'
New-Item -ItemType Directory -Force -Path $oebps, $metaInf | Out-Null

# mimetype (must be first entry, stored uncompressed)
$mimetype = 'application/epub+zip'
Set-Content -LiteralPath (Join-Path $stage 'mimetype') -Value $mimetype -NoNewline

# META-INF/container.xml
Copy-Item -LiteralPath (Join-Path $TutorialDir 'container.xml') -Destination $metaInf

# OEBPS/style.css
Copy-Item -LiteralPath (Join-Path $TutorialDir 'style.css') -Destination $oebps

# Static chapters
foreach ($ch in $Chapters) {
    if (-not $ch.Generated) {
        $src = Join-Path $TutorialDir $ch.File
        if (-not (Test-Path -LiteralPath $src)) {
            throw "Missing tutorial chapter: $src"
        }
        Copy-Item -LiteralPath $src -Destination $oebps
    }
}

# Generated ch00
$ch00Content = Build-WhatsNew
$ch00Path = Join-Path $oebps 'ch00-whats-new.xhtml'
Set-Content -LiteralPath $ch00Path -Value $ch00Content -NoNewline -Encoding UTF8

# content.opf
$opfContent = Build-Opf
Set-Content -LiteralPath (Join-Path $oebps 'content.opf') -Value $opfContent -NoNewline -Encoding UTF8

# toc.ncx
$ncxContent = Build-Ncx
Set-Content -LiteralPath (Join-Path $oebps 'toc.ncx') -Value $ncxContent -NoNewline -Encoding UTF8

# cover.png
New-CoverImage (Join-Path $oebps 'cover.png')

# ---------------------------------------------------------------------------
# Zip with mimetype stored uncompressed
# ---------------------------------------------------------------------------
if (-not (Test-Path $SamplesDir)) { New-Item -ItemType Directory -Force -Path $SamplesDir | Out-Null }
$epubPath = Join-Path $SamplesDir 'welcome.epub'
if (Test-Path $epubPath) { Remove-Item -LiteralPath $epubPath -Force }

$zip = [System.IO.Compression.ZipFile]::Open($epubPath, [System.IO.Compression.ZipArchiveMode]::Create)

# mimetype: stored, no compression, must be first
$mtEntry = $zip.CreateEntry('mimetype', [System.IO.Compression.CompressionLevel]::NoCompression)
$mtWriter = New-Object System.IO.StreamWriter($mtEntry.Open())
$mtWriter.Write($mimetype)
$mtWriter.Flush()
$mtWriter.Close()

# Everything else: deflate
function Add-FileToZip($zip, $sourcePath, $entryName) {
    [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
        $zip, $sourcePath, $entryName,
        [System.IO.Compression.CompressionLevel]::Optimal) | Out-Null
}

Add-FileToZip $zip (Join-Path $metaInf 'container.xml') 'META-INF/container.xml'
Add-FileToZip $zip (Join-Path $oebps 'content.opf') 'OEBPS/content.opf'
Add-FileToZip $zip (Join-Path $oebps 'toc.ncx') 'OEBPS/toc.ncx'
Add-FileToZip $zip (Join-Path $oebps 'style.css') 'OEBPS/style.css'
Add-FileToZip $zip (Join-Path $oebps 'cover.png') 'OEBPS/cover.png'
foreach ($ch in $Chapters) {
    Add-FileToZip $zip (Join-Path $oebps $ch.File) "OEBPS/$($ch.File)"
}

$zip.Dispose()

# Cleanup stage
Remove-Item -Recurse -Force $stage

# ---------------------------------------------------------------------------
# Report
# ---------------------------------------------------------------------------
$size = [math]::Round((Get-Item -LiteralPath $epubPath).Length / 1KB, 0)
Write-Host "Built: $epubPath ($size KB)"
Write-Host "Chapters: $($Chapters.Count)"
Write-Host "Version: $BuildTag"
