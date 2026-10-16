# SPDX-License-Identifier: MIT
# Pester 5 tests for the driver scripts. They run on any OS:
#   Invoke-Pester driver/tests

BeforeAll {
    $driverRoot = Join-Path $PSScriptRoot '..'
    . (Join-Path $driverRoot 'scripts/AudioProgramCable.Common.ps1')
    $script:inf = Get-Content -Raw (Join-Path $driverRoot 'AudioProgramCable.inx')
    $script:header = Get-Content -Raw (Join-Path $driverRoot 'src/cable_config.h')
    $script:pwshPath = (Get-Process -Id $PID).Path

    function Get-HeaderValue([string]$Name) {
        if ($script:header -notmatch "#define\s+$Name\s+L?""?([^""\s]+)") { throw "$Name not found in cable_config.h" }
        $Matches[1]
    }
}

Describe 'Driver settings agree across INF, driver and scripts' {
    It 'uses the same cable count limits' {
        Get-HeaderValue 'CABLE_COUNT_DEFAULT' | Should -Be $script:ApcCableCountDefault
        Get-HeaderValue 'CABLE_COUNT_MIN' | Should -Be $script:ApcCableCountMin
        Get-HeaderValue 'CABLE_COUNT_MAX' | Should -Be $script:ApcCableCountMax
        $script:inf | Should -Match "HKR,Parameters,$(Get-HeaderValue 'CABLE_COUNT_VALUE_NAME'),0x00010003,$script:ApcCableCountDefault\b"
    }

    It 'validates Set-CableCount -Count with the driver limits' {
        $command = Get-Command (Join-Path $driverRoot 'scripts/Set-CableCount.ps1')
        $range = $command.Parameters['Count'].Attributes | Where-Object { $_ -is [ValidateRange] }
        $range.MinRange | Should -Be $script:ApcCableCountMin
        $range.MaxRange | Should -Be $script:ApcCableCountMax
    }

    It 'declares an interface template for <_>' -ForEach @(
        'CABLE_TEMPLATE_RENDER_WAVE', 'CABLE_TEMPLATE_RENDER_TOPO',
        'CABLE_TEMPLATE_CAPTURE_WAVE', 'CABLE_TEMPLATE_CAPTURE_TOPO'
    ) {
        $template = Get-HeaderValue $_
        $script:inf | Should -Match "KSNAME_$template\s*=\s*""$template"""
        $script:inf | Should -Match "AddInterface\s*=\s*%KSCATEGORY_AUDIO%,\s*%KSNAME_$template%,\s*AudioProgramCable\.I\.$template\s"
    }

    It 'installs the service and hardware ID the scripts look for' {
        $script:inf | Should -Match "AddService\s*=\s*$script:ApcServiceName,"
        $script:inf | Should -Match ",\s*$([regex]::Escape($script:ApcHardwareId))\s"
    }
}

Describe 'Invoke-Native' {
    It 'returns an allowed exit code' {
        Invoke-Native $script:pwshPath @('-NoProfile', '-Command', 'exit 3') -AllowedExitCodes @(0, 3) |
            Should -Be 3
    }

    It 'throws on any other exit code' {
        { Invoke-Native $script:pwshPath @('-NoProfile', '-Command', 'exit 5') } |
            Should -Throw '*failed with exit code 5.'
    }
}

Describe 'Resolve-ApcPackageDir' {
    BeforeEach {
        $script:savedRoot = $script:ApcDriverRoot
        $script:ApcDriverRoot = (New-Item (Join-Path $TestDrive ([guid]::NewGuid())) -ItemType Directory).FullName
    }
    AfterEach { $script:ApcDriverRoot = $script:savedRoot }

    It 'returns -PackageDir as a full path' {
        Resolve-ApcPackageDir -PackageDir $TestDrive | Should -Be ([IO.Path]::GetFullPath($TestDrive))
    }

    It 'finds the INF in the folder above scripts' {
        New-Item (Join-Path $script:ApcDriverRoot 'AudioProgramCable.inf') | Out-Null
        Resolve-ApcPackageDir | Should -Be $script:ApcDriverRoot
    }

    It 'explains how to fix a missing package' {
        { Resolve-ApcPackageDir } | Should -Throw '*Pass -PackageDir*'
    }
}

Describe 'Find-KitTool' {
    It 'prefers the restored NuGet kit' {
        $script:savedRoot = $script:ApcDriverRoot
        try {
            $script:ApcDriverRoot = $TestDrive
            $tool = Join-Path $TestDrive "packages\Microsoft.Windows.SDK.CPP.$script:ApcKitVersion\c\bin\$script:ApcKitBin\x64\signtool.exe"
            New-Item $tool -ItemType File -Force | Out-Null
            Find-KitTool 'signtool.exe' | Should -Be $tool
        } finally {
            $script:ApcDriverRoot = $script:savedRoot
        }
    }
}

Describe 'Get-ApcDevices' {
    BeforeAll {
        if (-not (Get-Command Get-PnpDevice -ErrorAction SilentlyContinue)) {
            function global:Get-PnpDevice { param([switch]$PresentOnly) $null = $PresentOnly }
        }
    }

    It 'returns only devices with the cable hardware ID' {
        Mock Get-PnpDevice {
            [pscustomobject]@{ InstanceId = 'ROOT\MEDIA\0000'; HardwareID = @('ROOT\AudioProgramCable') }
            [pscustomobject]@{ InstanceId = 'ROOT\MEDIA\0001'; HardwareID = @('ROOT\OtherDevice') }
            [pscustomobject]@{ InstanceId = 'ROOT\MEDIA\0002'; HardwareID = $null }
        }
        (Get-ApcDevices).InstanceId | Should -Be 'ROOT\MEDIA\0000'
    }
}

Describe 'Initialize-ApcSetupApi' {
    It 'compiles the SetupAPI wrapper' {
        Initialize-ApcSetupApi
        'ApcSetupApi' -as [type] | Should -Not -BeNullOrEmpty
    }
}
