# SPDX-License-Identifier: MIT
<#
.SYNOPSIS
    Creates a self-signed code-signing certificate for test-signing the driver.

.DESCRIPTION
    The certificate is created in the current user's personal store
    (Cert:\CurrentUser\My) and its public part is exported to a .cer file.
    It is for test machines only: Windows loads drivers signed with it only
    when test signing is on (see Enable-TestSigning.ps1).

    With -Trust (needs administrator), the .cer is also added to the local
    machine's Trusted Root and Trusted Publishers stores, so the driver
    installs without a "Windows can't verify the publisher" prompt.

.EXAMPLE
    .\New-TestCertificate.ps1
    Creates the certificate and writes AudioProgramCableTest.cer next to the
    driver build output. Prints the thumbprint to pass to Sign-Driver.ps1.
#>
[CmdletBinding()]
param(
    [string]$Subject = 'CN=Audio Program Cable Test Signing (TEST ONLY)',
    [string]$CerPath,
    [int]$ValidMonths = 12,
    [switch]$Trust
)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\AudioProgramCable.Common.ps1"

if (-not $CerPath) {
    $CerPath = Join-Path $script:ApcDriverRoot 'build\AudioProgramCableTest.cer'
}
$CerPath = [IO.Path]::GetFullPath($CerPath)
New-Item -ItemType Directory -Force -Path (Split-Path $CerPath -Parent) | Out-Null

$existing = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert |
    Where-Object { $_.Subject -eq $Subject -and $_.NotAfter -gt (Get-Date).AddDays(7) -and $_.HasPrivateKey } |
    Sort-Object NotAfter -Descending | Select-Object -First 1

if ($existing) {
    $cert = $existing
    Write-Host "Reusing existing test certificate $($cert.Thumbprint) (expires $($cert.NotAfter.ToString('yyyy-MM-dd')))."
} else {
    $cert = New-SelfSignedCertificate -Type CodeSigningCert -Subject $Subject `
        -CertStoreLocation Cert:\CurrentUser\My -KeyAlgorithm RSA -KeyLength 3072 `
        -HashAlgorithm SHA256 -KeyExportPolicy Exportable -NotAfter (Get-Date).AddMonths($ValidMonths)
    Write-Host "Created test certificate $($cert.Thumbprint) (expires $($cert.NotAfter.ToString('yyyy-MM-dd')))."
}

Export-Certificate -Cert $cert -FilePath $CerPath -Type CERT | Out-Null
Write-Host "Exported public certificate to $CerPath"

if ($Trust) {
    Assert-Administrator
    foreach ($store in @('Root', 'TrustedPublisher')) {
        Import-Certificate -FilePath $CerPath -CertStoreLocation "Cert:\LocalMachine\$store" | Out-Null
        Write-Host "Added to Cert:\LocalMachine\$store"
    }
}

[pscustomobject]@{
    Thumbprint = $cert.Thumbprint
    Subject    = $cert.Subject
    NotAfter   = $cert.NotAfter
    CerPath    = $CerPath
}
