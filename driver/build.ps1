# SPDX-License-Identifier: MIT
<#
.SYNOPSIS
    Builds AudioProgramCable.sys with the WDK and assembles a driver package.

.DESCRIPTION
    Run from a Visual Studio 2022 Developer PowerShell (or any shell where
    msbuild.exe is on PATH, or where vswhere can find Visual Studio with the
    C++ workload). The Windows SDK and WDK come from NuGet with
    -RestorePackages, so no separate WDK install is needed.

    Output (driver\build\package by default):
      AudioProgramCable.sys / .inf / .cat   the driver package
      AudioProgramCableTest.cer             with -TestSign, the public test certificate
      scripts\, TESTING.md, LICENSES\       everything needed on the test PC
      BUILD-INFO.json                       commit, signing details, SHA-256 hashes

.EXAMPLE
    .\build.ps1 -RestorePackages -TestSign
.EXAMPLE
    .\build.ps1 -Configuration Debug
#>
[CmdletBinding()]
param(
    [ValidateSet('Debug', 'Release')][string]$Configuration = 'Release',
    [string]$OutputDir,
    [switch]$RestorePackages,
    [switch]$TestSign,
    # Deletes the signing key after signing, so the certificate can sign
    # nothing else (used by CI).
    [switch]$EphemeralCertificate
)
$ErrorActionPreference = 'Stop'
$driverRoot = $PSScriptRoot
. (Join-Path $driverRoot 'scripts\AudioProgramCable.Common.ps1')

$kitVersion = $script:ApcKitVersion
$kitBin = $script:ApcKitBin
$packages = Join-Path $driverRoot 'packages'
if (-not $OutputDir) { $OutputDir = Join-Path $driverRoot 'build\package' }
$OutputDir = [IO.Path]::GetFullPath($OutputDir)

function Find-NuGet {
    $cmd = Get-Command nuget.exe -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    $local = Join-Path $driverRoot 'build\tools\nuget.exe'
    if (-not (Test-Path -LiteralPath $local)) {
        New-Item -ItemType Directory -Force -Path (Split-Path $local -Parent) | Out-Null
        Write-Host 'Downloading nuget.exe'
        Invoke-WebRequest -Uri 'https://dist.nuget.org/win-x86-commandline/latest/nuget.exe' -OutFile $local -UseBasicParsing
    }
    return $local
}

function Find-MSBuild {
    $cmd = Get-Command msbuild.exe -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path -LiteralPath $vswhere) {
        $found = & $vswhere -latest -products * -requires Microsoft.Component.MSBuild -find 'MSBuild\**\Bin\amd64\MSBuild.exe' | Select-Object -First 1
        if ($found) { return $found }
    }
    throw 'msbuild.exe not found. Install Visual Studio 2022 with "Desktop development with C++", or run from a Developer PowerShell.'
}

if ($RestorePackages) {
    $nuget = Find-NuGet
    Invoke-Native $nuget @('restore', (Join-Path $driverRoot 'packages.config'), '-PackagesDirectory', $packages,
        '-NonInteractive', '-Source', 'https://api.nuget.org/v3/index.json') | Out-Null
}

$wdk = Join-Path $packages "Microsoft.Windows.WDK.x64.$kitVersion\c"
$sdk = Join-Path $packages "Microsoft.Windows.SDK.CPP.$kitVersion\c"
if (Test-Path -LiteralPath $wdk) {
    # The NuGet kits are not on PATH; the StampInf and InfVerif build steps
    # invoke the tools by name.
    $env:PATH = "$wdk\bin\$kitBin\x64;$wdk\bin\$kitBin\x86;$wdk\tools\$kitBin\x64;$sdk\bin\$kitBin\x64;$env:PATH"
} else {
    Write-Host 'NuGet WDK not restored; using the installed WDK. Pass -RestorePackages to use the pinned kit.'
}

$msbuild = Find-MSBuild
$project = Join-Path $driverRoot 'AudioProgramCable.vcxproj'
Invoke-Native $msbuild @($project, '/m', '/nologo', '/t:Build', "/p:Configuration=$Configuration", '/p:Platform=x64',
    '/p:PreferredToolArchitecture=x64', '/v:minimal') | Out-Null

$outDir = Join-Path $driverRoot "build\x64\$Configuration"
$intDir = Join-Path $driverRoot "build\obj\x64\$Configuration"
$sys = Join-Path $outDir 'AudioProgramCable.sys'
if (-not (Test-Path -LiteralPath $sys -PathType Leaf)) { throw "Build finished but $sys is missing." }
$inf = @(
    (Join-Path $outDir 'AudioProgramCable\AudioProgramCable.inf'),
    (Join-Path $outDir 'AudioProgramCable.inf'),
    (Join-Path $intDir 'AudioProgramCable.inf')
) | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
if (-not $inf) { throw 'Build finished but the stamped AudioProgramCable.inf is missing.' }

if (Test-Path -LiteralPath $OutputDir) { Remove-Item -LiteralPath $OutputDir -Recurse -Force }
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
Copy-Item -LiteralPath $sys, $inf -Destination $OutputDir
$pdb = Join-Path $outDir 'AudioProgramCable.pdb'
if (Test-Path -LiteralPath $pdb) {
    New-Item -ItemType Directory -Force -Path (Join-Path $OutputDir 'symbols') | Out-Null
    Copy-Item -LiteralPath $pdb -Destination (Join-Path $OutputDir 'symbols')
}

$signing = 'unsigned'
$thumbprint = $null
$expires = $null
if ($TestSign) {
    $cerPath = Join-Path $OutputDir 'AudioProgramCableTest.cer'
    $subject = if ($EphemeralCertificate) { 'CN=Audio Program Cable CI Test Signing (TEST ONLY)' } else { 'CN=Audio Program Cable Test Signing (TEST ONLY)' }
    $cert = & (Join-Path $driverRoot 'scripts\New-TestCertificate.ps1') -Subject $subject -CerPath $cerPath -ValidMonths 6
    try {
        & (Join-Path $driverRoot 'scripts\Sign-Driver.ps1') -PackageDir $OutputDir -Thumbprint $cert.Thumbprint
    } finally {
        if ($EphemeralCertificate) {
            Remove-Item -LiteralPath "Cert:\CurrentUser\My\$($cert.Thumbprint)" -DeleteKey
        }
    }
    $signing = 'self-signed test certificate (TEST ONLY)'
    $thumbprint = $cert.Thumbprint
    $expires = $cert.NotAfter.ToUniversalTime().ToString('o')
} else {
    $inf2cat = Find-KitTool -Name 'Inf2Cat.exe' -Arch x86
    Invoke-Native $inf2cat @("/driver:$OutputDir", '/os:10_VB_X64,10_CO_X64,10_NI_X64,10_GE_X64', '/uselocaltime') | Out-Null
}

Copy-Item -LiteralPath (Join-Path $driverRoot 'scripts') -Destination (Join-Path $OutputDir 'scripts') -Recurse
Copy-Item -LiteralPath (Join-Path $driverRoot 'LICENSES') -Destination (Join-Path $OutputDir 'LICENSES') -Recurse
Copy-Item -LiteralPath (Join-Path $driverRoot 'TESTING.md') -Destination $OutputDir

$commit = $null
try { $commit = (& git -C $driverRoot rev-parse HEAD 2>$null) } catch { Write-Verbose "git unavailable: $_" }
$hashes = [ordered]@{}
foreach ($name in @('AudioProgramCable.sys', 'AudioProgramCable.inf', 'AudioProgramCable.cat', 'AudioProgramCableTest.cer')) {
    $file = Join-Path $OutputDir $name
    if (Test-Path -LiteralPath $file) { $hashes[$name] = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant() }
}
[ordered]@{
    commit                = $commit
    configuration         = $Configuration
    kit                   = $kitVersion
    signing               = $signing
    certificateThumbprint = $thumbprint
    certificateExpires    = $expires
    sha256                = $hashes
} | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $OutputDir 'BUILD-INFO.json') -Encoding UTF8

Write-Host "Driver package: $OutputDir"
if (-not $TestSign) { Write-Host 'The package is unsigned. Use -TestSign to sign it for a test PC.' }
