# Silent install and uninstall of the mixer-only setup.
# Settings planted under %APPDATA%\Audio Program must survive a silent uninstall.

param(
    [Parameter(Mandatory = $true)][string]$Version,
    [string]$OutputDir = "dist",
    [string]$BuiltBinary = "target\release\audio-program.exe"
)

$ErrorActionPreference = "Stop"

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) {
        throw $Message
    }
}

function Assert-Checksums([string]$Dir, [string[]]$ExpectedNames) {
    $sumsPath = Join-Path $Dir "SHA256SUMS.txt"
    Assert-True (Test-Path -LiteralPath $sumsPath) "missing SHA256SUMS.txt"
    $raw = [System.IO.File]::ReadAllText($sumsPath)
    Assert-True (-not $raw.Contains([char]0xFEFF)) "SHA256SUMS.txt has a BOM"
    Assert-True (-not $raw.Contains("`r")) "SHA256SUMS.txt must use LF line endings"
    Assert-True $raw.EndsWith("`n") "SHA256SUMS.txt must end with a newline"
    $lines = $raw.TrimEnd("`n").Split("`n")
    $names = @()
    foreach ($line in $lines) {
        Assert-True ($line -match '^([a-f0-9]{64})  (\S+)$') "bad checksum line: $line"
        $hash = $Matches[1]
        $name = $Matches[2]
        $file = Join-Path $Dir $name
        Assert-True (Test-Path -LiteralPath $file) "checksum lists missing file $name"
        $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $file).Hash.ToLower()
        Assert-True ($actual -eq $hash) "checksum mismatch for $name"
        $names += $name
    }
    $want = @($ExpectedNames | Sort-Object) -join "|"
    $got = @($names | Sort-Object) -join "|"
    Assert-True ($want -eq $got) "checksum files [$got] expected [$want]"
}

function Open-UninstallKey([Microsoft.Win32.RegistryHive]$Hive, [string]$SubName) {
    $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey($Hive, [Microsoft.Win32.RegistryView]::Registry64)
    return $base.OpenSubKey($SubName)
}

function Get-ExeFromUninstallString([string]$Text) {
    if ($Text -match '^"([^"]+)"') {
        return $Matches[1]
    }
    return ($Text -split ' ', 2)[0]
}

function Get-WindowsSubsystem([string]$Path) {
    $bytes = [System.IO.File]::ReadAllBytes($Path)
    Assert-True ($bytes.Length -ge 0x40 -and $bytes[0] -eq 0x4D -and $bytes[1] -eq 0x5A) "not a PE file: $Path"
    $pe = [BitConverter]::ToInt32($bytes, 0x3C)
    Assert-True ($pe -gt 0 -and ($pe + 94) -lt $bytes.Length) "PE header is truncated: $Path"
    return [BitConverter]::ToUInt16($bytes, $pe + 24 + 68)
}

function Assert-GuiStaysOpen([string]$Exe) {
    # IMAGE_SUBSYSTEM_WINDOWS_GUI. A console subsystem (3) flashes a terminal.
    $subsystem = Get-WindowsSubsystem $Exe
    Assert-True ($subsystem -eq 2) "mixer is a console program (subsystem $subsystem)"
    $log = Join-Path (Split-Path -Parent $Exe) "audio-program-startup.log"
    if (Test-Path -LiteralPath $log) {
        Remove-Item -LiteralPath $log -Force
    }
    $proc = Start-Process -FilePath $Exe -WorkingDirectory (Split-Path -Parent $Exe) -PassThru
    try {
        $exited = $proc.WaitForExit(15000)
        $note = ""
        if (Test-Path -LiteralPath $log) {
            $note = [System.IO.File]::ReadAllText($log)
        }
        if ($exited) {
            throw "GUI exited immediately (exit $($proc.ExitCode)). $note"
        }
        if ($note -ne "") {
            throw "GUI logged a startup failure: $note"
        }
    } finally {
        if (-not $proc.HasExited) {
            Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
            $proc.WaitForExit()
        }
    }
}

function Assert-HeadlessStarts([string]$Exe, [string]$LogDir) {
    $outFile = Join-Path $LogDir "headless-out.txt"
    $errFile = Join-Path $LogDir "headless-err.txt"
    $proc = Start-Process -FilePath $Exe -ArgumentList "--headless" -PassThru `
        -RedirectStandardOutput $outFile -RedirectStandardError $errFile
    $stayedUp = -not $proc.WaitForExit(20000)
    if ($stayedUp) {
        Stop-Process -Id $proc.Id -Force
        $proc.WaitForExit()
        return
    }
    $code = $proc.ExitCode
    $text = ""
    if (Test-Path -LiteralPath $outFile) {
        $text += [System.IO.File]::ReadAllText($outFile)
    }
    if (Test-Path -LiteralPath $errFile) {
        $text += [System.IO.File]::ReadAllText($errFile)
    }
    if ($code -eq 0) {
        return
    }
    # Windows has no virtual cables yet. Headless reports that and exits 1.
    if ($code -eq 1 -and $text -match "audio-program:") {
        return
    }
    throw "installed mixer did not start in --headless mode (exit $code): $text"
}

if (-not [System.IO.Path]::IsPathRooted($OutputDir)) {
    $OutputDir = Join-Path (Get-Location) $OutputDir
}
$OutputDir = (Resolve-Path -LiteralPath $OutputDir).Path
if (-not [System.IO.Path]::IsPathRooted($BuiltBinary)) {
    $BuiltBinary = Join-Path (Get-Location) $BuiltBinary
}
$BuiltBinary = (Resolve-Path -LiteralPath $BuiltBinary).Path

$logDir = Join-Path (Get-Location) "logs"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

$setupName = "AudioProgram-Setup-$Version.exe"
$zipName = "AudioProgram-$Version-portable.zip"
$setup = Join-Path $OutputDir $setupName
$zipPath = Join-Path $OutputDir $zipName
Assert-True (Test-Path -LiteralPath $setup) "missing $setup"
Assert-True (Test-Path -LiteralPath $zipPath) "missing $zipPath"
Assert-Checksums $OutputDir @($setupName, $zipName)

$header = New-Object byte[] 2
$stream = [System.IO.File]::OpenRead($setup)
try {
    [void]$stream.Read($header, 0, 2)
} finally {
    $stream.Dispose()
}
Assert-True (($header[0] -eq 0x4D) -and ($header[1] -eq 0x5A)) "setup file is not a Windows executable"

$extract = Join-Path $logDir "zip"
if (Test-Path -LiteralPath $extract) {
    Remove-Item -LiteralPath $extract -Recurse -Force
}
Expand-Archive -LiteralPath $zipPath -DestinationPath $extract
$zipExe = Join-Path $extract "AudioProgram\audio-program.exe"
$zipLicense = Join-Path $extract "AudioProgram\LICENSE"
Assert-True (Test-Path -LiteralPath $zipExe) "portable zip is missing AudioProgram\audio-program.exe"
Assert-True (Test-Path -LiteralPath $zipLicense) "portable zip is missing LICENSE"
$zipFiles = @(Get-ChildItem -LiteralPath $extract -Recurse -File)
Assert-True ($zipFiles.Count -eq 2) "portable zip should contain the mixer and the license only"

$builtHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $BuiltBinary).Hash
$zipHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $zipExe).Hash
Assert-True ($builtHash -eq $zipHash) "portable zip mixer does not match the build"

$issPath = Join-Path $PSScriptRoot "audio-program.iss"
$issText = [System.IO.File]::ReadAllText($issPath)
Assert-True ($issText -match 'AppId=\{\{([0-9A-Fa-f-]+)\}(\r?\n|$)') "AppId not found in the installer script"
$guid = $Matches[1]
$uninstallSub = "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{$guid}_is1"

$appDir = Join-Path $env:ProgramFiles "Audio Program"
$installedExe = Join-Path $appDir "audio-program.exe"
$commonPrograms = [Environment]::GetFolderPath([Environment+SpecialFolder]::CommonPrograms)
$group = Join-Path $commonPrograms "Audio Program"
$appLink = Join-Path $group "Audio Program.lnk"
$uninstallLink = Join-Path $group "Uninstall Audio Program.lnk"
$publicDesktop = Join-Path $env:PUBLIC "Desktop\Audio Program.lnk"
$userDesktop = Join-Path ([Environment]::GetFolderPath("Desktop")) "Audio Program.lnk"
$userPrograms = Join-Path ([Environment]::GetFolderPath("Programs")) "Audio Program"
$settings = Join-Path $env:APPDATA "Audio Program"
$settingsFile = Join-Path $settings "settings-keep.txt"
$programFilesX86 = ${env:ProgramFiles(x86)}
$wrongDir = Join-Path $programFilesX86 "Audio Program"

try {
    $installLog = Join-Path $logDir "install.log"
    $install = Start-Process -FilePath $setup -ArgumentList @(
        "/VERYSILENT"
        "/SUPPRESSMSGBOXES"
        "/NORESTART"
        "/LOG=$installLog"
    ) -Wait -PassThru
    if ($install.ExitCode -ne 0) {
        $tail = ""
        if (Test-Path -LiteralPath $installLog) {
            $tail = (Get-Content -LiteralPath $installLog -Tail 40) -join "`n"
        }
        throw "install failed with exit code $($install.ExitCode)`n$tail"
    }

    Assert-True (Test-Path -LiteralPath $installedExe) "mixer exe missing after install"
    Assert-True (Test-Path -LiteralPath (Join-Path $appDir "LICENSE")) "LICENSE missing after install"
    Assert-True (Test-Path -LiteralPath $appLink) "Start menu shortcut missing"
    Assert-True (Test-Path -LiteralPath $uninstallLink) "Start menu uninstall shortcut missing"
    Assert-True (-not (Test-Path -LiteralPath $publicDesktop)) "desktop shortcut was created without being asked"
    Assert-True (-not (Test-Path -LiteralPath $userDesktop)) "per-user desktop shortcut was created"
    Assert-True (-not (Test-Path -LiteralPath $userPrograms)) "install was not all-users"
    Assert-True (-not (Test-Path -LiteralPath $wrongDir)) "mixer was installed into Program Files (x86)"
    $hklmKey = Open-UninstallKey ([Microsoft.Win32.RegistryHive]::LocalMachine) $uninstallSub
    $hkcuKey = Open-UninstallKey ([Microsoft.Win32.RegistryHive]::CurrentUser) $uninstallSub
    Assert-True ($null -ne $hklmKey) "all-users uninstall entry missing"
    Assert-True ($null -eq $hkcuKey) "per-user uninstall entry was written"

    $displayVersion = [string]$hklmKey.GetValue("DisplayVersion")
    Assert-True ($displayVersion -eq $Version) "display version was $displayVersion"
    Assert-True ([string]::IsNullOrEmpty([string]$hklmKey.GetValue("Publisher"))) "publisher should be empty"
    $location = ([string]$hklmKey.GetValue("InstallLocation")).TrimEnd('\')
    Assert-True ($location -eq $appDir) "install location was $location"

    $shortcut = (New-Object -ComObject WScript.Shell).CreateShortcut($appLink)
    Assert-True ($shortcut.TargetPath -eq $installedExe) "Start menu shortcut target was $($shortcut.TargetPath)"

    $installedHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installedExe).Hash
    Assert-True ($installedHash -eq $builtHash) "installed mixer does not match the build"

    Assert-HeadlessStarts $installedExe $logDir
    Assert-GuiStaysOpen $installedExe
    Get-Process -Name "audio-program" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500

    New-Item -ItemType Directory -Force -Path $settings | Out-Null
    [System.IO.File]::WriteAllText($settingsFile, "keep")

    $unins = Get-ExeFromUninstallString ([string]$hklmKey.GetValue("UninstallString"))
    Assert-True (Test-Path -LiteralPath $unins) "uninstaller missing: $unins"
    $uninstallLog = Join-Path $logDir "uninstall.log"
    $removed = Start-Process -FilePath $unins -ArgumentList @(
        "/VERYSILENT"
        "/SUPPRESSMSGBOXES"
        "/NORESTART"
        "/LOG=$uninstallLog"
    ) -Wait -PassThru
    if ($removed.ExitCode -ne 0) {
        $tail = ""
        if (Test-Path -LiteralPath $uninstallLog) {
            $tail = (Get-Content -LiteralPath $uninstallLog -Tail 40) -join "`n"
        }
        throw "uninstall failed with exit code $($removed.ExitCode)`n$tail"
    }

    $deadline = (Get-Date).AddSeconds(30)
    while ((Test-Path -LiteralPath $appDir) -and (Get-Date) -lt $deadline) {
        Start-Sleep -Seconds 1
    }

    Assert-True (-not (Test-Path -LiteralPath $installedExe)) "mixer exe still present after uninstall"
    Assert-True (-not (Test-Path -LiteralPath $appDir)) "install directory still present after uninstall"
    Assert-True (-not (Test-Path -LiteralPath $appLink)) "Start menu shortcut still present"
    Assert-True (-not (Test-Path -LiteralPath $uninstallLink)) "uninstall shortcut still present"
    Assert-True (-not (Test-Path -LiteralPath $group)) "Start menu folder still present"
    Assert-True ($null -eq (Open-UninstallKey ([Microsoft.Win32.RegistryHive]::LocalMachine) $uninstallSub)) "uninstall entry still present"
    Assert-True (Test-Path -LiteralPath $settingsFile) "silent uninstall deleted settings"
} finally {
    Get-Process -Name "audio-program" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
}
