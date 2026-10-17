# SPDX-License-Identifier: MIT
<#
.SYNOPSIS
    Installs the test-signed Sonicanvas smoke-test driver (Root\SonicanvasAudio)
    with the Audio Program install script.

.EXAMPLE
    .\Install-Sonicanvas.ps1
#>
[CmdletBinding()]
param(
    [string]$PackageDir,
    [switch]$Force
)
$ErrorActionPreference = 'Stop'
$scripts = Join-Path $PSScriptRoot 'scripts'
if (-not (Test-Path -LiteralPath (Join-Path $scripts 'Install-Driver.ps1'))) {
    $scripts = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\scripts'))
}
if (-not $PackageDir) {
    $PackageDir = if (Test-Path -LiteralPath (Join-Path $PSScriptRoot 'SonicanvasAudio.inf')) { $PSScriptRoot }
                  else { [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\build\smoke-test\sonicanvas')) }
}
& (Join-Path $scripts 'Install-Driver.ps1') -PackageDir $PackageDir -BaseName 'SonicanvasAudio' -HardwareId 'Root\SonicanvasAudio' -Force:$Force
