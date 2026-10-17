# SPDX-License-Identifier: MIT
<#
.SYNOPSIS
    Turns Windows test-signing mode on (or off with -Disable).

.DESCRIPTION
    Test-signed drivers load only when the boot option TESTSIGNING is on. The
    change takes effect after a reboot, and Windows then shows "Test Mode" in
    the corner of the desktop.

    Secure Boot blocks this setting. Turn Secure Boot off in the PC's UEFI
    setup (or the VM's settings) first. If BitLocker protects the system
    drive, the script suspends it for one reboot so the boot change does not
    trigger a recovery-key prompt.

.EXAMPLE
    .\Enable-TestSigning.ps1 -Restart
.EXAMPLE
    .\Enable-TestSigning.ps1 -Disable
#>
[CmdletBinding()]
param(
    [switch]$Disable,
    [switch]$Restart
)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\AudioProgramCable.Common.ps1"
Assert-Administrator

$bcdedit = "$env:SystemRoot\System32\bcdedit.exe"
$wanted = if ($Disable) { 'off' } else { 'on' }

if (-not $Disable -and (Get-SecureBootEnabled)) {
    throw 'Secure Boot is on, so Windows ignores test signing. Turn Secure Boot off in UEFI/VM settings, boot Windows, and run this again.'
}

$currentlyOn = Get-TestSigningEnabled
if ($currentlyOn -eq (-not $Disable)) {
    Write-Host "Test signing is already $wanted."
    return
}

$bitlocker = Get-Command Get-BitLockerVolume -ErrorAction SilentlyContinue
if ($bitlocker) {
    $volume = Get-BitLockerVolume -MountPoint $env:SystemDrive -ErrorAction SilentlyContinue
    if ($volume -and $volume.ProtectionStatus -eq 'On') {
        Write-Host "Suspending BitLocker on $env:SystemDrive for one reboot."
        Suspend-BitLocker -MountPoint $env:SystemDrive -RebootCount 1 | Out-Null
    }
}

Invoke-Native $bcdedit @('/set', 'testsigning', $wanted) | Out-Null
Write-Host "Test signing set to $wanted. Reboot to apply."

if ($Restart) {
    Restart-Computer -Force
}
