# Descarga el binario de PDFium para la plataforma pedida, verifica su hash
# contra scripts/pdfium.lock y lo deja en vendor/pdfium/<target>/ (PLAN.md §5, M0).
#
# Uso:  pwsh scripts/fetch-pdfium.ps1 [-Target win-x64] [-Record]
#   -Target : win-x64 | mac-arm64 | mac-x64 | linux-x64 | android-arm64
#   -Record : registra el hash descargado en pdfium.lock (solo para actualizar la versión pineada)

param(
    [ValidateSet("win-x64", "mac-arm64", "mac-x64", "linux-x64", "android-arm64")]
    [string]$Target = "win-x64",
    [switch]$Record
)

$ErrorActionPreference = "Stop"
$lockPath = Join-Path $PSScriptRoot "pdfium.lock"
$lock = Get-Content $lockPath -Raw | ConvertFrom-Json

$asset = "pdfium-$Target.tgz"
$url = "https://github.com/bblanchon/pdfium-binaries/releases/download/$($lock.tag)/$asset"
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) "pdfshard-$asset"

Write-Host "Descargando $url"
Invoke-WebRequest -Uri $url -OutFile $tmp

$hash = (Get-FileHash $tmp -Algorithm SHA256).Hash.ToLower()
$expected = $lock.sha256.$Target
if ($Record) {
    $lock.sha256.$Target = $hash
    $lock | ConvertTo-Json | Set-Content $lockPath -Encoding utf8
    Write-Host "Hash registrado para ${Target}: $hash"
} elseif ($expected -and $expected -ne $hash) {
    throw "Hash de $asset NO coincide. Esperado $expected, obtenido $hash"
} elseif (-not $expected) {
    throw "No hay hash registrado para $Target en pdfium.lock (use -Record para registrarlo)"
}

$extractDir = Join-Path ([System.IO.Path]::GetTempPath()) "pdfshard-pdfium-$Target"
if (Test-Path $extractDir) { Remove-Item -Recurse -Force $extractDir }
New-Item -ItemType Directory -Force $extractDir | Out-Null
tar -xzf $tmp -C $extractDir

$lib = Get-ChildItem $extractDir -Recurse -Include "pdfium.dll", "libpdfium.so", "libpdfium.dylib" | Select-Object -First 1
if (-not $lib) { throw "No se encontró la biblioteca PDFium dentro de $asset" }

$destDir = Join-Path (Split-Path $PSScriptRoot -Parent) "vendor/pdfium/$Target"
New-Item -ItemType Directory -Force $destDir | Out-Null
Copy-Item $lib.FullName $destDir -Force
Write-Host "PDFium ($Target) listo en $destDir  [tag $($lock.tag)]"
