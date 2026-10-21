param(
  [Parameter(Mandatory = $true)]
  [string]$Version
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "../..")
$ExePath = Join-Path $RepoRoot "src-tauri/target/release/llm-wiki.exe"
$PdfiumPath = Join-Path $RepoRoot "src-tauri/pdfium/pdfium.dll"
$DistRoot = Join-Path $RepoRoot "dist-portable"
$PortableRoot = Join-Path $DistRoot "Micelya-Desktop-$Version-windows-x64-portable"
$ZipPath = Join-Path $DistRoot "Micelya-Desktop-$Version-windows-x64-portable.zip"

if (!(Test-Path $ExePath)) {
  throw "Tauri executable was not found at $ExePath"
}
if (!(Test-Path $PdfiumPath)) {
  throw "PDFium DLL was not found at $PdfiumPath"
}
if (Test-Path $PortableRoot) {
  Remove-Item -Recurse -Force $PortableRoot
}
if (Test-Path $ZipPath) {
  Remove-Item -Force $ZipPath
}
New-Item -ItemType Directory -Force $PortableRoot | Out-Null

Copy-Item $ExePath (Join-Path $PortableRoot "Micelya Desktop.exe")

New-Item -ItemType Directory -Force (Join-Path $PortableRoot "pdfium") | Out-Null
Copy-Item $PdfiumPath (Join-Path $PortableRoot "pdfium/pdfium.dll")


@"
Micelya Desktop Windows Portable

Run "Micelya Desktop.exe" from this folder. Keep the pdfium/ folder next to the executable.

This portable package does not install start-menu shortcuts or auto-update hooks. It still stores app data in the normal Micelya Desktop application data directory.
"@ | Set-Content -Encoding UTF8 (Join-Path $PortableRoot "README-portable.txt")

Compress-Archive -Path (Join-Path $PortableRoot "*") -DestinationPath $ZipPath -CompressionLevel Optimal
Write-Host "Created $ZipPath"
