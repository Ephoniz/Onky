# SPDX-License-Identifier: MIT
<#
.SYNOPSIS
    Shows the state of the Audio Program Cable driver: test signing, the
    device, the cable-count setting, and the audio endpoints Windows created.

.EXAMPLE
    .\Get-CableStatus.ps1
#>
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\AudioProgramCable.Common.ps1"

$parameters = "HKLM:\SYSTEM\CurrentControlSet\Services\$script:ApcServiceName\Parameters"
$configured = $null
if (Test-Path -LiteralPath $parameters) {
    $configured = (Get-ItemProperty -LiteralPath $parameters -Name CableCount -ErrorAction SilentlyContinue).CableCount
}

$devices = @(Get-ApcDevices)
$driverVersion = $null
$present = $devices | Where-Object { $_.Present } | Select-Object -First 1
if ($present) {
    $driverVersion = (Get-PnpDeviceProperty -InstanceId $present.InstanceId -KeyName 'DEVPKEY_Device_DriverVersion' -ErrorAction SilentlyContinue).Data
}

$endpoints = @(Get-PnpDevice -Class AudioEndpoint -ErrorAction SilentlyContinue |
    Where-Object { $_.Present -and $_.FriendlyName -like '*Audio Program*' } |
    Sort-Object FriendlyName)

[pscustomobject]@{
    TestSigning        = Get-TestSigningEnabled
    SecureBoot         = Get-SecureBootEnabled
    Devices            = $devices.Count
    DriverVersion      = $driverVersion
    CableCountSetting  = if ($null -ne $configured) { $configured } else { "(not set, default $script:ApcCableCountDefault)" }
    RenderEndpoints    = @($endpoints | Where-Object { $_.FriendlyName -notlike 'Microphone*' }).Count
    CaptureEndpoints   = @($endpoints | Where-Object { $_.FriendlyName -like 'Microphone*' }).Count
} | Format-List | Out-Host

if ($devices.Count -gt 0) {
    Write-Host 'Device:'
    $devices | Format-Table -AutoSize Status, Problem, Present, InstanceId | Out-Host
    $bad = $devices | Where-Object { $_.Present -and $_.Status -ne 'OK' }
    foreach ($device in $bad) {
        $code = (Get-PnpDeviceProperty -InstanceId $device.InstanceId -KeyName 'DEVPKEY_Device_ProblemCode' -ErrorAction SilentlyContinue).Data
        Write-Warning "Device problem code $code. 52 = signature not trusted (test signing off or Secure Boot on); 10 = driver failed to start; 28 = no driver installed."
    }
}

if ($endpoints.Count -gt 0) {
    Write-Host 'Audio endpoints:'
    $endpoints | Format-Table -AutoSize Status, FriendlyName | Out-Host
} else {
    Write-Host 'No Audio Program endpoints found.'
}
