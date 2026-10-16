# Build AudioProgram-Setup-<version>.exe and the portable mixer zip.
# Checksums are written afterwards by scripts/write-sha256sums.sh.

param(
    [Parameter(Mandatory = $true)][string]$Version,
    [Parameter(Mandatory = $true)][string]$Iscc,
    [string]$Binary = "target\release\audio-program.exe",
    [string]$OutputDir = "dist"
)

$ErrorActionPreference = "Stop"

if ($Version -notmatch '^(\d+)\.(\d+)\.(\d+)([-+][0-9A-Za-z.-]+)?$') {
    throw "Version must look like 0.1.0 or 0.1.0-alpha.1: $Version"
}
$versionInfo = "$($Matches[1]).$($Matches[2]).$($Matches[3]).0"

if (-not (Test-Path -LiteralPath $Binary)) {
    throw "Missing mixer binary: $Binary"
}
if (-not (Test-Path -LiteralPath $Iscc)) {
    throw "Missing ISCC: $Iscc"
}

$binaryAbs = (Resolve-Path -LiteralPath $Binary).Path
if (-not [System.IO.Path]::IsPathRooted($OutputDir)) {
    $OutputDir = Join-Path (Get-Location) $OutputDir
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$outAbs = (Resolve-Path -LiteralPath $OutputDir).Path

foreach ($path in @($binaryAbs, $outAbs)) {
    if ($path -match '\s') {
        throw "Paths with spaces are not supported: $path"
    }
}

$binaryForIss = $binaryAbs -replace '\\', '/'
$outForIss = $outAbs -replace '\\', '/'
$iss = Join-Path $PSScriptRoot "audio-program.iss"

& $Iscc "/DMyAppVersion=$Version" "/DMyAppVersionInfo=$versionInfo" "/DBinaryPath=$binaryForIss" "/DOutputDir=$outForIss" $iss
if ($LASTEXITCODE -ne 0) {
    throw "ISCC failed with exit code $LASTEXITCODE"
}

$setupPath = Join-Path $outAbs "AudioProgram-Setup-$Version.exe"
if (-not (Test-Path -LiteralPath $setupPath)) {
    throw "Installer was not produced: $setupPath"
}

$staging = Join-Path $outAbs "portable-staging"
$folder = Join-Path $staging "AudioProgram"
if (Test-Path -LiteralPath $staging) {
    Remove-Item -LiteralPath $staging -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $folder | Out-Null
Copy-Item -LiteralPath $binaryAbs -Destination (Join-Path $folder "audio-program.exe")
Copy-Item -LiteralPath (Join-Path $PSScriptRoot "..\..\LICENSE") -Destination (Join-Path $folder "LICENSE")

$zipPath = Join-Path $outAbs "AudioProgram-$Version-portable.zip"
if (Test-Path -LiteralPath $zipPath) {
    Remove-Item -LiteralPath $zipPath -Force
}
Compress-Archive -Path $folder -DestinationPath $zipPath
Remove-Item -LiteralPath $staging -Recurse -Force
