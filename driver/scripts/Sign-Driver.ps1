# SPDX-License-Identifier: MIT
<#
.SYNOPSIS
    Test-signs a driver package folder: the .sys, a fresh catalog (.cat), and
    the catalog signature.

.DESCRIPTION
    The folder must contain AudioProgramCable.sys and AudioProgramCable.inf
    (driver\build.ps1 produces it at driver\build\package). Inf2Cat creates
    AudioProgramCable.cat from the INF, then signtool signs the .sys and .cat
    with a certificate from Cert:\CurrentUser\My.

    The PE signature is not part of the file hash stored in the catalog, so
    signing the .sys before or after Inf2Cat gives the same catalog.

.EXAMPLE
    .\New-TestCertificate.ps1
    .\Sign-Driver.ps1
#>
[CmdletBinding()]
param(
    [string]$PackageDir,
    [string]$Thumbprint,
    [string]$Subject = 'CN=Audio Program Cable Test Signing (TEST ONLY)',
    [string]$BaseName = 'AudioProgramCable',
    [string]$OsList = '10_VB_X64,10_CO_X64,10_NI_X64,10_GE_X64',
    [switch]$CopyCertificate
)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\AudioProgramCable.Common.ps1"

$PackageDir = Resolve-ApcPackageDir -PackageDir $PackageDir -BaseName $BaseName
$sys = Join-Path $PackageDir "$BaseName.sys"
$inf = Join-Path $PackageDir "$BaseName.inf"
$cat = Join-Path $PackageDir "$BaseName.cat"
foreach ($file in @($sys, $inf)) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "Missing $file. Build the driver first (driver\build.ps1)." }
}

if ($Thumbprint) {
    $cert = Get-Item -LiteralPath "Cert:\CurrentUser\My\$Thumbprint" -ErrorAction Stop
} else {
    $cert = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert |
        Where-Object { $_.Subject -eq $Subject -and $_.NotAfter -gt (Get-Date) -and $_.HasPrivateKey } |
        Sort-Object NotAfter -Descending | Select-Object -First 1
    if (-not $cert) { throw "No certificate '$Subject' in Cert:\CurrentUser\My. Run New-TestCertificate.ps1 first." }
}
if (-not $cert.HasPrivateKey) { throw "Certificate $($cert.Thumbprint) has no private key." }

$signtool = Find-KitTool -Name 'signtool.exe' -Arch x64
$inf2cat  = Find-KitTool -Name 'Inf2Cat.exe' -Arch x86

Write-Host "Signing $sys"
Invoke-Native $signtool @('sign', '/fd', 'SHA256', '/s', 'My', '/sha1', $cert.Thumbprint, $sys) | Out-Null

if (Test-Path -LiteralPath $cat) { Remove-Item -LiteralPath $cat -Force }
Write-Host "Creating catalog for $OsList"
Invoke-Native $inf2cat @("/driver:$PackageDir", "/os:$OsList", '/uselocaltime') | Out-Null
if (-not (Test-Path -LiteralPath $cat -PathType Leaf)) { throw "Inf2Cat did not create $cat." }

Write-Host "Signing $cat"
Invoke-Native $signtool @('sign', '/fd', 'SHA256', '/s', 'My', '/sha1', $cert.Thumbprint, $cat) | Out-Null

if ($CopyCertificate) {
    $cer = Join-Path $PackageDir "$($BaseName)Test.cer"
    Export-Certificate -Cert $cert -FilePath $cer -Type CERT | Out-Null
    Write-Host "Exported public certificate to $cer"
}

Write-Host "Signed package in $PackageDir with certificate $($cert.Thumbprint)."
