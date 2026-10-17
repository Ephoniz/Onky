# SPDX-License-Identifier: MIT
<#
.SYNOPSIS
    Installs the test-signed Audio Program Cable driver and creates its device.

.DESCRIPTION
    1. Trusts the package's test certificate (*.cer in the package folder) by
       adding it to the local machine's Trusted Root and Trusted Publishers.
    2. Adds the package to the Driver Store with "pnputil /add-driver".
    3. Creates the ROOT\AudioProgramCable device if it does not exist yet and
       installs the driver on it. pnputil cannot create root devices, so this
       uses the same SetupAPI calls as devcon. With -Devcon, devcon.exe does
       steps 2 and 3 instead.

    Needs an elevated 64-bit PowerShell and test signing turned on
    (Enable-TestSigning.ps1).

.EXAMPLE
    .\Install-Driver.ps1
.EXAMPLE
    .\Install-Driver.ps1 -Devcon 'C:\Program Files (x86)\Windows Kits\10\Tools\10.0.28000.0\x64\devcon.exe'
#>
[CmdletBinding()]
param(
    [string]$PackageDir,
    [string]$BaseName = 'AudioProgramCable',
    [string]$HardwareId = 'ROOT\AudioProgramCable',
    [string]$Devcon,
    [switch]$SkipCertificate,
    [switch]$Force
)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\AudioProgramCable.Common.ps1"
Assert-Administrator
Assert-64BitProcess

$PackageDir = Resolve-ApcPackageDir -PackageDir $PackageDir -BaseName $BaseName
$inf = Join-Path $PackageDir "$BaseName.inf"
foreach ($ext in @('inf', 'sys', 'cat')) {
    $file = Join-Path $PackageDir "$BaseName.$ext"
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "Missing $file. The package must contain the .inf, .sys and signed .cat." }
}

if (-not (Get-TestSigningEnabled)) {
    $message = 'Test signing is off, so Windows will refuse to start a test-signed driver (Device Manager code 52). Run Enable-TestSigning.ps1 and reboot first.'
    if ($Force) { Write-Warning $message } else { throw "$message Use -Force to install anyway." }
}

if (-not $SkipCertificate) {
    $cers = @(Get-ChildItem -LiteralPath $PackageDir -Filter '*.cer' -File)
    if ($cers.Count -eq 0) {
        Write-Warning 'No .cer file in the package folder; skipping certificate trust. Windows may ask whether to trust the publisher.'
    }
    foreach ($cer in $cers) {
        foreach ($store in @('Root', 'TrustedPublisher')) {
            $imported = Import-Certificate -FilePath $cer.FullName -CertStoreLocation "Cert:\LocalMachine\$store"
            Write-Host "Trusted $($cer.Name) ($($imported.Thumbprint)) in Cert:\LocalMachine\$store"
        }
    }
}

$reboot = $false
$pnputil = "$env:SystemRoot\System32\pnputil.exe"

if ($Devcon) {
    if (-not (Test-Path -LiteralPath $Devcon -PathType Leaf)) { throw "devcon.exe not found at $Devcon" }
    $verb = if (@(Get-ApcDevices -HardwareId $HardwareId | Where-Object { $_.Present }).Count -gt 0) { 'update' } else { 'install' }
    Write-Host "Running devcon $verb"
    $code = Invoke-Native $Devcon @($verb, $inf, $HardwareId) -AllowedExitCodes @(0, 1)
    if ($code -eq 1) { $reboot = $true }
} else {
    Write-Host 'Adding the driver package to the Driver Store (pnputil /add-driver)'
    $code = Invoke-Native $pnputil @('/add-driver', $inf) -AllowedExitCodes @(0, 259, 3010)
    if ($code -eq 3010) { $reboot = $true }

    Initialize-ApcSetupApi
    # A removed device can linger as "not present"; it cannot take a driver.
    if (@(Get-ApcDevices -HardwareId $HardwareId | Where-Object { $_.Present }).Count -eq 0) {
        Write-Host "Creating root device $HardwareId"
        [ApcSetupApi]::CreateRootDevice($inf, $HardwareId)
    } else {
        Write-Host "Device $HardwareId already exists; updating its driver"
    }
    Write-Host 'Installing the driver on the device'
    if ([ApcSetupApi]::InstallDriver($inf, $HardwareId)) { $reboot = $true }
}

Start-Sleep -Seconds 3
$devices = @(Get-ApcDevices -HardwareId $HardwareId | Where-Object { $_.Present })
$devices | Format-Table -AutoSize Status, Problem, FriendlyName, InstanceId | Out-Host

if ($reboot) {
    Write-Warning 'Windows asked for a reboot to finish installing. Reboot, then run Get-CableStatus.ps1.'
} elseif ($devices | Where-Object { $_.Status -ne 'OK' }) {
    Write-Warning 'The device was installed but is not running. Run Get-CableStatus.ps1 and see TESTING.md, "If something goes wrong".'
} else {
    Write-Host 'Installed. Open Settings > System > Sound to see the cables.'
}
