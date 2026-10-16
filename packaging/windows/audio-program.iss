; Mixer-only installer. Virtual cables are not installed by this setup.
; Stable id. An upgrade replaces this install and leaves settings in place.

#ifndef MyAppVersion
  #error "Pass /DMyAppVersion=... to ISCC"
#endif
#ifndef MyAppVersionInfo
  #error "Pass /DMyAppVersionInfo=... to ISCC"
#endif
#ifndef BinaryPath
  #error "Pass /DBinaryPath=... to ISCC"
#endif
#ifndef OutputDir
  #error "Pass /DOutputDir=... to ISCC"
#endif

#define MyAppName "Audio Program"
#define MyAppExeName "audio-program.exe"

[Setup]
AppId={{F605ED42-60D8-49A9-A2E2-98BC6BB1579F}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
VersionInfoVersion={#MyAppVersionInfo}
VersionInfoDescription=Audio Program mixer
VersionInfoProductName={#MyAppName}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile=..\..\LICENSE
OutputDir={#OutputDir}
OutputBaseFilename=AudioProgram-Setup-{#MyAppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
UninstallDisplayIcon={app}\{#MyAppExeName}
CloseApplications=yes
RestartApplications=no
SetupLogging=yes

[Files]
Source: "{#BinaryPath}"; DestDir: "{app}"; DestName: "{#MyAppExeName}"
Source: "..\..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"; Flags: unchecked

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Open Audio Program"; Flags: postinstall nowait skipifsilent

[Code]
function SettingsDirectory: String;
begin
    Result := ExpandConstant('{userappdata}\Audio Program');
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
    if CurUninstallStep <> usPostUninstall then
        Exit;
    if not DirExists(SettingsDirectory) then
        Exit;
    // A silent uninstall keeps settings. The default button is No.
    if UninstallSilent then
        Exit;
    if MsgBox('Also delete your settings?', mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES then
        DelTree(SettingsDirectory, True, True, True);
end;
