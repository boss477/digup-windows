$ErrorActionPreference = 'Stop'
$root = Join-Path $PSScriptRoot 'vendor\ocr-fixtures'
New-Item -ItemType Directory -Path $root -Force | Out-Null
Add-Type -AssemblyName System.Drawing
$bitmap = New-Object Drawing.Bitmap 1600,400
$graphics = [Drawing.Graphics]::FromImage($bitmap)
$font = New-Object Drawing.Font 'Arial',48
try {
    $graphics.Clear([Drawing.Color]::White)
    $graphics.DrawString('SEARCHABLE LEASE CLAUSE 4827', $font, [Drawing.Brushes]::Black, 40, 120)
    $bitmap.Save((Join-Path $root 'ocr.png'), [Drawing.Imaging.ImageFormat]::Png)
    $jpeg = New-Object IO.MemoryStream
    $bitmap.Save($jpeg, [Drawing.Imaging.ImageFormat]::Jpeg)
    $imageBytes = $jpeg.ToArray()
    $jpeg.Dispose()
} finally { $graphics.Dispose(); $font.Dispose(); $bitmap.Dispose() }
# Generate a two-page PDF: ordinary text on page 1, raster-only OCR text on page 2.
$pdf = New-Object IO.MemoryStream
$offsets = @{}
function Write-Ascii([string]$value) { $bytes = [Text.Encoding]::ASCII.GetBytes($value); $pdf.Write($bytes,0,$bytes.Length) }
function Write-Object([int]$number,[string]$body) { $offsets[$number] = $pdf.Position; Write-Ascii "$number 0 obj`n$body`nendobj`n" }
Write-Ascii "%PDF-1.4`n"
Write-Object 1 '<< /Type /Catalog /Pages 2 0 R >>'
Write-Object 2 '<< /Type /Pages /Kids [3 0 R 6 0 R] /Count 2 >>'
Write-Object 3 '<< /Type /Page /Parent 2 0 R /MediaBox [0 0 800 400] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>'
Write-Object 4 '<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>'
$stream = 'BT /F1 12 Tf 30 300 Td (Ordinary first page) Tj ET'
Write-Object 5 "<< /Length $($stream.Length) >>`nstream`n$stream`nendstream"
Write-Object 6 '<< /Type /Page /Parent 2 0 R /MediaBox [0 0 800 400] /Resources << /XObject << /Im1 7 0 R >> >> /Contents 8 0 R >>'
$offsets[7] = $pdf.Position
Write-Ascii "7 0 obj`n<< /Type /XObject /Subtype /Image /Width 1600 /Height 400 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length $($imageBytes.Length) >>`nstream`n"
$pdf.Write($imageBytes,0,$imageBytes.Length)
Write-Ascii "`nendstream`nendobj`n"
$stream = 'q 800 0 0 200 0 100 cm /Im1 Do Q'
Write-Object 8 "<< /Length $($stream.Length) >>`nstream`n$stream`nendstream"
$xref = $pdf.Position
Write-Ascii "xref`n0 9`n0000000000 65535 f `n"
foreach ($number in 1..8) { Write-Ascii ('{0:0000000000} 00000 n ' -f $offsets[$number]); Write-Ascii "`n" }
Write-Ascii "trailer`n<< /Size 9 /Root 1 0 R >>`nstartxref`n$xref`n%%EOF"
[IO.File]::WriteAllBytes((Join-Path $root 'scanned.pdf'), $pdf.ToArray())
$pdf.Dispose()
& (Join-Path $PSScriptRoot 'build.ps1') -TestsOnly
$env:DIGUP_OCR_FIXTURE_ROOT = $root
& (Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe') test --locked ocr_fixture_search -- --ignored --nocapture
if ($LASTEXITCODE -ne 0) { throw 'OCR integration test failed' }
