# SPDX-License-Identifier: MIT
<#
.SYNOPSIS
    Removes the Sonicanvas smoke-test device, its driver package and its test
    certificate.

.EXAMPLE
    .\Uninstall-Sonicanvas.ps1
#>
[CmdletBinding()]
param(
    [string]$PackageDir
)
$ErrorActionPreference = 'Stop'
$scripts = Join-Path $PSScriptRoot 'scripts'
if (-not (Test-Path -LiteralPath (Join-Path $scripts 'Uninstall-Driver.ps1'))) {
    $scripts = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\scripts'))
}
if (-not $PackageDir) {
    $PackageDir = if (Test-Path -LiteralPath (Join-Path $PSScriptRoot 'SonicanvasAudio.inf')) { $PSScriptRoot }
                  else { [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\build\smoke-test\sonicanvas')) }
}
& (Join-Path $scripts 'Uninstall-Driver.ps1') -PackageDir $PackageDir -BaseName 'SonicanvasAudio' -HardwareId 'Root\SonicanvasAudio' `
    -CertificateSubject 'CN=Sonicanvas Driver TEST ONLY' -RemoveCertificate
