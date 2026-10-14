# SPDX-License-Identifier: MIT
<#
.SYNOPSIS
    Builds a test-signed Sonicanvas WaveRT driver package from a pinned commit,
    for use as a known-good smoke test of the test PC.

.DESCRIPTION
    Clones https://github.com/open-string/sonicanvas-windows-driver (MIT) at a
    fixed commit, runs its own build-driver.ps1 and package-driver-test.ps1,
    and copies the result plus the Audio Program install scripts into one
    folder. Nothing from Sonicanvas is vendored into this repository.

    Needs git, Visual Studio 2022 with the C++ workload (msbuild) and nuget.exe
    on PATH.

.EXAMPLE
    .\Build-SonicanvasPackage.ps1
#>
[CmdletBinding()]
param(
    [string]$Commit = 'd38f16a0fa848a7b85563b3e5fb65b82dd7721db',
    [string]$Repository = 'https://github.com/open-string/sonicanvas-windows-driver.git',
    [string]$WorkDir,
    [string]$OutputDir
)
$ErrorActionPreference = 'Stop'
$here = $PSScriptRoot
$driverRoot = [IO.Path]::GetFullPath((Join-Path $here '..\..'))
. (Join-Path $driverRoot 'scripts\AudioProgramCable.Common.ps1')

if (-not $WorkDir) { $WorkDir = Join-Path $here 'work' }
if (-not $OutputDir) { $OutputDir = Join-Path $driverRoot 'build\smoke-test\sonicanvas' }
$WorkDir = [IO.Path]::GetFullPath($WorkDir)
$OutputDir = [IO.Path]::GetFullPath($OutputDir)
$source = Join-Path $WorkDir 'sonicanvas-windows-driver'

$git = (Get-Command git.exe -ErrorAction Stop).Source
if (-not (Test-Path -LiteralPath (Join-Path $source '.git'))) {
    New-Item -ItemType Directory -Force -Path $WorkDir | Out-Null
    Invoke-Native $git @('init', '--quiet', $source) | Out-Null
    Invoke-Native $git @('-C', $source, 'remote', 'add', 'origin', $Repository) | Out-Null
}
Invoke-Native $git @('-C', $source, 'fetch', '--quiet', '--depth', '1', 'origin', $Commit) | Out-Null
Invoke-Native $git @('-C', $source, 'checkout', '--quiet', '--force', '--detach', 'FETCH_HEAD') | Out-Null
$head = (& $git -C $source rev-parse HEAD).Trim()
if ($head -ne $Commit) { throw "Checked out $head, expected $Commit." }
Write-Host "Sonicanvas source at $head"

& (Join-Path $source 'windows\scripts\build-driver.ps1') -Configuration Release -RestorePackages
& (Join-Path $source 'windows\scripts\package-driver-test.ps1') -Configuration Release

$package = Join-Path $source 'build\windows-driver-test\driver'
foreach ($name in @('SonicanvasAudio.sys', 'SonicanvasAudio.inf', 'SonicanvasAudio.cat', 'SonicanvasDriverTest.cer')) {
    if (-not (Test-Path -LiteralPath (Join-Path $package $name) -PathType Leaf)) { throw "Sonicanvas package is missing $name." }
}

if (Test-Path -LiteralPath $OutputDir) { Remove-Item -LiteralPath $OutputDir -Recurse -Force }
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
Get-ChildItem -LiteralPath $package -Force | Where-Object { $_.Name -ne '.sonicanvas-driver-package' } |
    Copy-Item -Destination $OutputDir -Recurse
New-Item -ItemType Directory -Force -Path (Join-Path $OutputDir 'scripts') | Out-Null
foreach ($name in @('AudioProgramCable.Common.ps1', 'Install-Driver.ps1', 'Uninstall-Driver.ps1', 'Enable-TestSigning.ps1')) {
    Copy-Item -LiteralPath (Join-Path $driverRoot "scripts\$name") -Destination (Join-Path $OutputDir 'scripts')
}
foreach ($name in @('Install-Sonicanvas.ps1', 'Uninstall-Sonicanvas.ps1', 'README.md')) {
    Copy-Item -LiteralPath (Join-Path $here $name) -Destination $OutputDir
}
Write-Host "Sonicanvas smoke-test package: $OutputDir"
