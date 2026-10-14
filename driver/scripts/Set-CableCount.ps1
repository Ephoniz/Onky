# SPDX-License-Identifier: MIT
<#
.SYNOPSIS
    Sets how many cables the driver creates (1 to 16, default 8).

.DESCRIPTION
    Writes the CableCount value under
    HKLM\SYSTEM\CurrentControlSet\Services\AudioProgramCable\Parameters and
    restarts the device with "pnputil /restart-device" so the driver reads it.
    Values outside 1..16 are clamped by the driver. Close apps that use the
    cables first, or the restart waits for a reboot.

.EXAMPLE
    .\Set-CableCount.ps1 -Count 4
.EXAMPLE
    .\Set-CableCount.ps1 -Count 16 -NoRestart
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateRange(1, 16)][int]$Count,
    [switch]$NoRestart
)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\AudioProgramCable.Common.ps1"
Assert-Administrator

$service = "HKLM:\SYSTEM\CurrentControlSet\Services\$script:ApcServiceName"
if (-not (Test-Path -LiteralPath $service)) {
    throw "The $script:ApcServiceName service is not registered. Install the driver first (Install-Driver.ps1)."
}
$parameters = Join-Path $service 'Parameters'
if (-not (Test-Path -LiteralPath $parameters)) { New-Item -Path $parameters | Out-Null }
New-ItemProperty -LiteralPath $parameters -Name 'CableCount' -PropertyType DWord -Value $Count -Force | Out-Null
Write-Host "CableCount set to $Count."

if ($NoRestart) {
    Write-Host 'Restart the device (or reboot) to apply.'
    return
}

$pnputil = "$env:SystemRoot\System32\pnputil.exe"
$devices = @(Get-ApcDevices | Where-Object { $_.Present })
if ($devices.Count -eq 0) {
    Write-Warning 'No Audio Program Cable device is present. The new count applies when the device is next installed or started.'
    return
}
$reboot = $false
foreach ($device in $devices) {
    Write-Host "Restarting $($device.InstanceId)"
    $code = Invoke-Native $pnputil @('/restart-device', $device.InstanceId) -AllowedExitCodes @(0, 3010)
    if ($code -eq 3010) { $reboot = $true }
}
if ($reboot) {
    Write-Warning 'The device is in use and will restart at the next reboot. Close audio apps and run this again, or reboot.'
} else {
    Write-Host "Done. Sound settings should now list $Count cable(s)."
}
