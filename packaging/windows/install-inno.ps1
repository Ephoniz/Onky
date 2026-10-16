# Download Inno Setup 6.7.3 and install a portable compiler.
# Writes the ISCC.exe path to $env:RUNNER_TEMP/iscc-path.txt.

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

if (-not $env:RUNNER_TEMP) {
    $env:RUNNER_TEMP = $env:TEMP
}

$version = "6.7.3"
$expectedHash = "9c73c3bae7ed48d44112a0f48e66742c00090bdb5bef71d9d3c056c66e97b732"
$url = "https://github.com/jrsoftware/issrc/releases/download/is-6_7_3/innosetup-$version.exe"
$installer = Join-Path $env:RUNNER_TEMP "innosetup-$version.exe"
$dir = Join-Path $env:RUNNER_TEMP "innosetup-$version"
$log = Join-Path $env:RUNNER_TEMP "innosetup-install.log"

Invoke-WebRequest -Uri $url -OutFile $installer -UseBasicParsing
$hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installer).Hash.ToLower()
if ($hash -ne $expectedHash) {
    throw "Inno Setup installer hash mismatch: $hash"
}

New-Item -ItemType Directory -Force -Path $dir | Out-Null
$installArgs = @(
    "/PORTABLE=1"
    "/SP-"
    "/VERYSILENT"
    "/SUPPRESSMSGBOXES"
    "/NORESTART"
    "/CURRENTUSER"
    "/DIR=$dir"
    "/LOG=$log"
)
$proc = Start-Process -FilePath $installer -ArgumentList $installArgs -Wait -PassThru
if ($proc.ExitCode -ne 0) {
    if (Test-Path -LiteralPath $log) {
        Get-Content -LiteralPath $log -Tail 40
    }
    throw "Inno Setup installer exited $($proc.ExitCode)"
}

$found = Get-ChildItem -Path $dir -Filter ISCC.exe -Recurse -File | Select-Object -First 1
if (-not $found) {
    Get-ChildItem -Path $dir -Recurse -File | ForEach-Object { Write-Host $_.FullName }
    throw "ISCC.exe was not found under $dir"
}

$marker = Join-Path $env:RUNNER_TEMP "iscc-path.txt"
[System.IO.File]::WriteAllText($marker, $found.FullName)
Write-Host "ISCC $($found.FullName)"
