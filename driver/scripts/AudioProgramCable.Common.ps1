# SPDX-License-Identifier: MIT
# Shared helpers for the Audio Program Cable driver scripts. Dot-source it:
#   . "$PSScriptRoot\AudioProgramCable.Common.ps1"

Set-StrictMode -Version 3.0

$script:ApcServiceName = 'AudioProgramCable'
$script:ApcHardwareId  = 'ROOT\AudioProgramCable'
$script:ApcInfName     = 'AudioProgramCable.inf'
$script:ApcKitVersion  = '10.0.26100.6584'
$script:ApcKitBin      = '10.0.26100.0'
$script:ApcCableCountMin     = 1
$script:ApcCableCountMax     = 16
$script:ApcCableCountDefault = 8
$script:ApcDriverRoot  = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))

function Assert-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'Run this script from an elevated PowerShell (right-click PowerShell, "Run as administrator").'
    }
}

function Assert-64BitProcess {
    if (-not [Environment]::Is64BitProcess) {
        throw 'Run this script from 64-bit PowerShell. Device installation from a 32-bit process is redirected and fails.'
    }
}

# Finds a Windows SDK/WDK tool: first the NuGet kit restored by build.ps1, then
# an installed Windows Kit, then PATH.
function Find-KitTool {
    param(
        [Parameter(Mandatory)][string]$Name,
        [ValidateSet('x64', 'x86')][string]$Arch = 'x64'
    )
    $packages = Join-Path $script:ApcDriverRoot 'packages'
    $candidates = @(
        (Join-Path $packages "Microsoft.Windows.SDK.CPP.$script:ApcKitVersion\c\bin\$script:ApcKitBin\$Arch\$Name"),
        (Join-Path $packages "Microsoft.Windows.WDK.x64.$script:ApcKitVersion\c\bin\$script:ApcKitBin\$Arch\$Name"),
        (Join-Path $packages "Microsoft.Windows.WDK.x64.$script:ApcKitVersion\c\tools\$script:ApcKitBin\$Arch\$Name")
    )
    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
    }
    $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10'
    foreach ($sub in @('bin', 'tools')) {
        $root = Join-Path $kits $sub
        if (Test-Path -LiteralPath $root) {
            $found = Get-ChildItem -LiteralPath $root -Directory -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -match '^10\.' } |
                Sort-Object { [version]$_.Name } -Descending |
                ForEach-Object { Join-Path $_.FullName "$Arch\$Name" } |
                Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
                Select-Object -First 1
            if ($found) { return $found }
        }
    }
    $onPath = Get-Command $Name -ErrorAction SilentlyContinue
    if ($onPath) { return $onPath.Source }
    throw "Could not find $Name. Run driver\build.ps1 -RestorePackages once, or install the Windows SDK/WDK."
}

function Invoke-Native {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [string[]]$Arguments = @(),
        [int[]]$AllowedExitCodes = @(0)
    )
    Write-Verbose "> $FilePath $($Arguments -join ' ')"
    & $FilePath @Arguments | Out-Host
    $code = $LASTEXITCODE
    if ($AllowedExitCodes -notcontains $code) {
        throw "$([IO.Path]::GetFileName($FilePath)) failed with exit code $code."
    }
    return $code
}

function Get-ApcDevices {
    param([string]$HardwareId = $script:ApcHardwareId)
    Get-PnpDevice -PresentOnly:$false -ErrorAction SilentlyContinue |
        Where-Object { $_.HardwareID -and ($_.HardwareID -contains $HardwareId) }
}

function Get-ApcDriverPackages {
    param([string]$InfName = $script:ApcInfName)
    # Published names (oemNN.inf) of every copy of the package in the Driver Store.
    Get-WindowsDriver -Online -All -ErrorAction Stop |
        Where-Object { [IO.Path]::GetFileName($_.OriginalFileName) -ieq $InfName } |
        ForEach-Object { $_.Driver }
}

# The package folder is either the folder above scripts\ (the CI artifact
# layout) or driver\build\package (a local build).
function Resolve-ApcPackageDir {
    param([string]$PackageDir, [string]$BaseName = 'AudioProgramCable')
    if ($PackageDir) { return [IO.Path]::GetFullPath($PackageDir) }
    foreach ($candidate in @($script:ApcDriverRoot, (Join-Path $script:ApcDriverRoot 'build\package'))) {
        if (Test-Path -LiteralPath (Join-Path $candidate "$BaseName.inf") -PathType Leaf) { return $candidate }
    }
    throw "Could not find $BaseName.inf. Pass -PackageDir with the folder that holds $BaseName.inf, .sys and .cat."
}

function Get-TestSigningEnabled {
    $output = & "$env:SystemRoot\System32\bcdedit.exe" /enum '{current}' 2>$null
    return [bool]($output | Select-String -Pattern '^\s*testsigning\s+Yes\s*$' -Quiet)
}

function Get-SecureBootEnabled {
    try { return [bool](Confirm-SecureBootUEFI -ErrorAction Stop) }
    catch { return $false }
}

$script:ApcSetupApiSource = @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;

public static class ApcSetupApi
{
    [StructLayout(LayoutKind.Sequential)]
    private struct SP_DEVINFO_DATA
    {
        public int cbSize;
        public Guid ClassGuid;
        public int DevInst;
        public IntPtr Reserved;
    }

    private const int DICD_GENERATE_ID = 0x1;
    private const int SPDRP_HARDWAREID = 0x1;
    private const int DIF_REGISTERDEVICE = 0x19;
    private const int DIF_REMOVE = 0x5;
    private const int INSTALLFLAG_FORCE = 0x1;
    private static readonly IntPtr INVALID_HANDLE_VALUE = new IntPtr(-1);

    [DllImport("setupapi.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern bool SetupDiGetINFClassW(string infName, out Guid classGuid,
        StringBuilder className, int classNameSize, out int requiredSize);

    [DllImport("setupapi.dll", SetLastError = true)]
    private static extern IntPtr SetupDiCreateDeviceInfoList(ref Guid classGuid, IntPtr hwndParent);

    [DllImport("setupapi.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern bool SetupDiCreateDeviceInfoW(IntPtr deviceInfoSet, string deviceName,
        ref Guid classGuid, string deviceDescription, IntPtr hwndParent, int creationFlags,
        ref SP_DEVINFO_DATA deviceInfoData);

    [DllImport("setupapi.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern bool SetupDiSetDeviceRegistryPropertyW(IntPtr deviceInfoSet,
        ref SP_DEVINFO_DATA deviceInfoData, int property, byte[] propertyBuffer, int propertyBufferSize);

    [DllImport("setupapi.dll", SetLastError = true)]
    private static extern bool SetupDiCallClassInstaller(int installFunction, IntPtr deviceInfoSet,
        ref SP_DEVINFO_DATA deviceInfoData);

    [DllImport("setupapi.dll", SetLastError = true)]
    private static extern bool SetupDiDestroyDeviceInfoList(IntPtr deviceInfoSet);

    [DllImport("newdev.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern bool UpdateDriverForPlugAndPlayDevicesW(IntPtr hwndParent, string hardwareId,
        string fullInfPath, int installFlags, out bool rebootRequired);

    // Creates a ROOT\<CLASS>\NNNN device node with the given hardware ID, like
    // "devcon install". The node has no driver until InstallDriver runs.
    public static void CreateRootDevice(string infPath, string hardwareId)
    {
        Guid classGuid;
        int required;
        StringBuilder className = new StringBuilder(64);
        if (!SetupDiGetINFClassW(infPath, out classGuid, className, className.Capacity, out required))
            throw new Win32Exception(Marshal.GetLastWin32Error(), "SetupDiGetINFClass failed for " + infPath);

        IntPtr set = SetupDiCreateDeviceInfoList(ref classGuid, IntPtr.Zero);
        if (set == INVALID_HANDLE_VALUE)
            throw new Win32Exception(Marshal.GetLastWin32Error(), "SetupDiCreateDeviceInfoList failed");
        try
        {
            SP_DEVINFO_DATA data = new SP_DEVINFO_DATA();
            data.cbSize = Marshal.SizeOf(typeof(SP_DEVINFO_DATA));
            if (!SetupDiCreateDeviceInfoW(set, className.ToString(), ref classGuid, null, IntPtr.Zero,
                    DICD_GENERATE_ID, ref data))
                throw new Win32Exception(Marshal.GetLastWin32Error(), "SetupDiCreateDeviceInfo failed");

            byte[] ids = Encoding.Unicode.GetBytes(hardwareId + "\0\0");
            if (!SetupDiSetDeviceRegistryPropertyW(set, ref data, SPDRP_HARDWAREID, ids, ids.Length))
                throw new Win32Exception(Marshal.GetLastWin32Error(), "SetupDiSetDeviceRegistryProperty failed");

            if (!SetupDiCallClassInstaller(DIF_REGISTERDEVICE, set, ref data))
                throw new Win32Exception(Marshal.GetLastWin32Error(), "DIF_REGISTERDEVICE failed");
        }
        finally
        {
            SetupDiDestroyDeviceInfoList(set);
        }
    }

    // Installs (or updates) the driver from infPath on every device with this
    // hardware ID. Returns true when Windows asks for a reboot.
    public static bool InstallDriver(string infPath, string hardwareId)
    {
        bool reboot;
        if (!UpdateDriverForPlugAndPlayDevicesW(IntPtr.Zero, hardwareId, infPath, INSTALLFLAG_FORCE, out reboot))
            throw new Win32Exception(Marshal.GetLastWin32Error(), "UpdateDriverForPlugAndPlayDevices failed");
        return reboot;
    }
}
'@

function Initialize-ApcSetupApi {
    if (-not ('ApcSetupApi' -as [type])) {
        Add-Type -TypeDefinition $script:ApcSetupApiSource -Language CSharp
    }
}
