# SPDX-License-Identifier: MIT
<#
.SYNOPSIS
    Removes the Audio Program Cable device and its driver package.

.DESCRIPTION
    1. Removes every ROOT\AudioProgramCable device ("pnputil /remove-device",
       or "devcon remove" with -Devcon).
    2. Deletes every AudioProgramCable.inf package from the Driver Store
       ("pnputil /delete-driver oemNN.inf /uninstall").
    3. With -RemoveCertificate, removes the test certificate from the local
       machine's Trusted Root and Trusted Publishers stores.

    Close apps that use the cables first. Test signing stays on; turn it off
    with Enable-TestSigning.ps1 -Disable.

.EXAMPLE
    .\Uninstall-Driver.ps1 -RemoveCertificate
#>
[CmdletBinding()]
param(
    [string]$BaseName = 'AudioProgramCable',
    [string]$HardwareId = 'ROOT\AudioProgramCable',
    [string]$CertificateSubject = 'CN=Audio Program Cable Test Signing (TEST ONLY)',
    [string]$PackageDir,
    [string]$Devcon,
    [switch]$RemoveCertificate
)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\AudioProgramCable.Common.ps1"
Assert-Administrator
Assert-64BitProcess

$pnputil = "$env:SystemRoot\System32\pnputil.exe"
$reboot = $false

$devices = @(Get-ApcDevices -HardwareId $HardwareId)
if ($devices.Count -eq 0) {
    Write-Host "No $HardwareId device found."
} elseif ($Devcon) {
    if (-not (Test-Path -LiteralPath $Devcon -PathType Leaf)) { throw "devcon.exe not found at $Devcon" }
    $code = Invoke-Native $Devcon @('remove', $HardwareId) -AllowedExitCodes @(0, 1)
    if ($code -eq 1) { $reboot = $true }
} else {
    foreach ($device in $devices) {
        Write-Host "Removing device $($device.InstanceId)"
        $code = Invoke-Native $pnputil @('/remove-device', $device.InstanceId) -AllowedExitCodes @(0, 3010)
        if ($code -eq 3010) { $reboot = $true }
    }
}

$packages = @(Get-ApcDriverPackages -InfName "$BaseName.inf")
if ($packages.Count -eq 0) {
    Write-Host "No $BaseName.inf package in the Driver Store."
}
foreach ($published in $packages) {
    Write-Host "Deleting driver package $published"
    $code = Invoke-Native $pnputil @('/delete-driver', $published, '/uninstall') -AllowedExitCodes @(0, 3010)
    if ($code -eq 3010) { $reboot = $true }
}

if ($RemoveCertificate) {
    $thumbprints = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    $dir = $null
    try { $dir = Resolve-ApcPackageDir -PackageDir $PackageDir -BaseName $BaseName } catch { Write-Verbose $_ }
    if ($dir) {
        foreach ($cer in Get-ChildItem -LiteralPath $dir -Filter '*.cer' -File) {
            $x509 = [Security.Cryptography.X509Certificates.X509Certificate2]::new($cer.FullName)
            [void]$thumbprints.Add($x509.Thumbprint)
        }
    }
    foreach ($store in @('Root', 'TrustedPublisher')) {
        Get-ChildItem "Cert:\LocalMachine\$store" |
            Where-Object { $thumbprints.Contains($_.Thumbprint) -or $_.Subject -eq $CertificateSubject } |
            ForEach-Object {
                Write-Host "Removing certificate $($_.Thumbprint) from Cert:\LocalMachine\$store"
                Remove-Item -LiteralPath $_.PSPath
            }
    }
}

if ($reboot) {
    Write-Warning 'Windows asked for a reboot to finish removing the driver.'
} else {
    Write-Host 'Uninstalled.'
}
