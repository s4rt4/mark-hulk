# Generates branded installer assets + the .md file-type icon.
# Run from anywhere; paths are absolute to the project.
Add-Type -AssemblyName System.Drawing

$root = "C:\laragon\www\mark-hulk"
$installerDir = Join-Path $root "src-tauri\installer"
New-Item -ItemType Directory -Force $installerDir | Out-Null

# ---- Palette ----
$brandDark = [System.Drawing.Color]::FromArgb(14, 44, 32)     # #0e2c20
$brandDeep = [System.Drawing.Color]::FromArgb(11, 35, 26)     # #0b231a
$mint      = [System.Drawing.Color]::FromArgb(149, 209, 175)  # #95d1af
$light     = [System.Drawing.Color]::FromArgb(234, 255, 244)  # #eafff4
$lightBg   = [System.Drawing.Color]::FromArgb(244, 247, 245)  # near-white
$inkGreen  = [System.Drawing.Color]::FromArgb(58, 102, 70)    # #3a6646

# =====================================================================
# 1) Multi-size square ICO for .md files (PNG-compressed entries)
# =====================================================================
function New-SquarePng([System.Drawing.Image]$src, [int]$size) {
  $bmp = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $g.Clear([System.Drawing.Color]::Transparent)
  $ratio = $src.Width / $src.Height
  $h = $size; $w = [int][math]::Round($size * $ratio)
  if ($w -gt $size) { $w = $size; $h = [int][math]::Round($size / $ratio) }
  # small margin so the doc isn't edge-to-edge
  $pad = [int]($size * 0.04)
  $h -= $pad * 2; $w = [int][math]::Round($h * $ratio)
  $x = [int](($size - $w) / 2); $y = [int](($size - $h) / 2)
  $g.DrawImage($src, $x, $y, $w, $h)
  $g.Dispose()
  $ms = New-Object System.IO.MemoryStream
  $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  return $ms.ToArray()
}

$srcDoc = [System.Drawing.Image]::FromFile((Join-Path $root "filetypes.png"))
$sizes = @(256, 128, 64, 48, 32, 24, 16)
$pngs = @{}
foreach ($s in $sizes) { $pngs[$s] = New-SquarePng $srcDoc $s }
$srcDoc.Dispose()

$bytes = New-Object System.Collections.Generic.List[byte]
function AddU16($v) { $bytes.AddRange([System.BitConverter]::GetBytes([uint16]$v)) }
function AddU32($v) { $bytes.AddRange([System.BitConverter]::GetBytes([uint32]$v)) }
AddU16 0; AddU16 1; AddU16 $sizes.Count
$offset = 6 + 16 * $sizes.Count
foreach ($s in $sizes) {
  $data = $pngs[$s]
  $dim = if ($s -ge 256) { 0 } else { $s }
  $bytes.Add([byte]$dim); $bytes.Add([byte]$dim)
  $bytes.Add([byte]0); $bytes.Add([byte]0)
  AddU16 1; AddU16 32
  AddU32 $data.Length; AddU32 $offset
  $offset += $data.Length
}
foreach ($s in $sizes) { $bytes.AddRange([byte[]]$pngs[$s]) }
[System.IO.File]::WriteAllBytes((Join-Path $root "src-tauri\filetypes.ico"), $bytes.ToArray())
Write-Host "filetypes.ico written ($($sizes.Count) sizes, $($bytes.Count) bytes)"

# =====================================================================
# 2) Branded BMPs for the installer (NSIS + WiX)
# =====================================================================
$logo = [System.Drawing.Image]::FromFile((Join-Path $root "mark-hulk.png"))

function New-Bmp([int]$w, [int]$h, [System.Drawing.Color]$bg, [scriptblock]$draw, [string]$path) {
  $bmp = New-Object System.Drawing.Bitmap($w, $h, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
  $g.Clear($bg)
  & $draw $g
  $g.Dispose()
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Bmp)
  $bmp.Dispose()
  Write-Host "wrote $path ($w x $h)"
}

function Draw-Logo($g, [int]$x, [int]$y, [int]$sz) {
  $g.DrawImage($logo, $x, $y, $sz, $sz)
}

# NSIS sidebar (welcome/finish) — 164 x 314, dark brand panel
New-Bmp 164 314 $brandDeep {
  param($g)
  $accent = New-Object System.Drawing.SolidBrush($mint)
  $g.FillRectangle($accent, 0, 0, 5, 314)
  Draw-Logo $g 34 46 96
  $f1 = New-Object System.Drawing.Font("Segoe UI", 17, [System.Drawing.FontStyle]::Bold)
  $f2 = New-Object System.Drawing.Font("Segoe UI", 9)
  $sf = New-Object System.Drawing.StringFormat; $sf.Alignment = "Center"
  $g.DrawString("Mark-Hulk", $f1, (New-Object System.Drawing.SolidBrush($light)), (New-Object System.Drawing.RectangleF(0, 158, 164, 30)), $sf)
  $g.DrawString("Markdown, redefined.", $f2, (New-Object System.Drawing.SolidBrush($mint)), (New-Object System.Drawing.RectangleF(0, 188, 164, 20)), $sf)
} (Join-Path $installerDir "nsis-sidebar.bmp")

# NSIS header — 150 x 57, light strip with logo + wordmark (blends with MUI header)
New-Bmp 150 57 $lightBg {
  param($g)
  Draw-Logo $g 8 9 40
  $f = New-Object System.Drawing.Font("Segoe UI", 11, [System.Drawing.FontStyle]::Bold)
  $g.DrawString("Mark-Hulk", $f, (New-Object System.Drawing.SolidBrush($inkGreen)), 54, 18)
} (Join-Path $installerDir "nsis-header.bmp")

# WiX banner — 493 x 58, light with wordmark left, logo right
New-Bmp 493 58 $lightBg {
  param($g)
  $f = New-Object System.Drawing.Font("Segoe UI", 13, [System.Drawing.FontStyle]::Bold)
  $f2 = New-Object System.Drawing.Font("Segoe UI", 9)
  $g.DrawString("Mark-Hulk", $f, (New-Object System.Drawing.SolidBrush($inkGreen)), 18, 10)
  $g.DrawString("Markdown, redefined.", $f2, (New-Object System.Drawing.SolidBrush($inkGreen)), 20, 33)
  Draw-Logo $g 437 9 40
} (Join-Path $installerDir "wix-banner.bmp")

# WiX dialog — 493 x 312, dark brand background
New-Bmp 493 312 $brandDeep {
  param($g)
  $panel = New-Object System.Drawing.SolidBrush($brandDark)
  $g.FillRectangle($panel, 0, 0, 170, 312)
  $accent = New-Object System.Drawing.SolidBrush($mint)
  $g.FillRectangle($accent, 170, 0, 3, 312)
  Draw-Logo $g 37 70 96
  $f1 = New-Object System.Drawing.Font("Segoe UI", 16, [System.Drawing.FontStyle]::Bold)
  $f2 = New-Object System.Drawing.Font("Segoe UI", 9)
  $sf = New-Object System.Drawing.StringFormat; $sf.Alignment = "Center"
  $g.DrawString("Mark-Hulk", $f1, (New-Object System.Drawing.SolidBrush($light)), (New-Object System.Drawing.RectangleF(0, 180, 170, 28)), $sf)
  $g.DrawString("Markdown, redefined.", $f2, (New-Object System.Drawing.SolidBrush($mint)), (New-Object System.Drawing.RectangleF(0, 208, 170, 20)), $sf)
} (Join-Path $installerDir "wix-dialog.bmp")

$logo.Dispose()
Write-Host "DONE"
