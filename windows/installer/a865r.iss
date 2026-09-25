; Native userspace tools. No kernel package, binding change or firmware flashing.
#define AppVersion "0.9.5"
[Setup]
#if VER >= EncodeVer(7, 0, 0)
SetupArchitecture=x64
#endif
AppId={{6C81FCAB-191E-4BB8-9585-D346954485D3}
AppName=Open Volar S
AppVersion={#AppVersion}
VersionInfoVersion=0.9.5.0
AppPublisher=Open Volar S contributors
DefaultDirName={localappdata}\Programs\A865R
DefaultGroupName=Open Volar S
AllowNoIcons=no
UsePreviousGroup=no
DisableDirPage=no
PrivilegesRequired=lowest
UsePreviousPrivileges=no
UsePreviousAppDir=no
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
MinVersion=10.0
OutputDir=..\..\dist
OutputBaseFilename=Open-Volar-S-Setup-{#AppVersion}-x64
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
LicenseFile=..\..\LICENSES.md
InfoBeforeFile=INSTALL-NOTES.txt
UninstallDisplayIcon={app}\player\live-tv-ruby.ico
CloseApplications=yes
RestartApplications=no
SetupLogging=yes
SetupIconFile=..\..\GUI\Windows\assets\app.ico

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Shortcuts:"

[Files]
Source: "..\..\GUI\Windows\assets\live-tv-ruby.ico"; DestDir: "{app}\player"; Flags: ignoreversion
#ifdef CrossPayload
Source: "{#CrossPayload}\player\*"; DestDir: "{app}\player"; Flags: ignoreversion
#else
Source: "..\..\GUI\Windows\live-tv.exe"; DestDir: "{app}\player"; Flags: ignoreversion
Source: "..\..\GUI\Windows\msvcp140.dll"; DestDir: "{app}\player"; Flags: ignoreversion
Source: "..\..\GUI\Windows\vcruntime140.dll"; DestDir: "{app}\player"; Flags: ignoreversion
Source: "..\..\GUI\Windows\vcruntime140_1.dll"; DestDir: "{app}\player"; Flags: ignoreversion
#endif
Source: "..\..\GUI\Windows\README.md"; DestDir: "{app}\player"; Flags: ignoreversion
#ifdef CrossPayload
Source: "{#CrossPayload}\debug\a865r-debug.exe"; DestDir: "{app}\debug"; Flags: ignoreversion
Source: "{#CrossPayload}\bin\a865rctl.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "{#CrossPayload}\wine\a865rctl"; DestDir: "{app}\wine"; Flags: ignoreversion; Check: IsWine
Source: "{#CrossPayload}\licenses\*"; DestDir: "{app}\licenses"; Flags: ignoreversion
#else
Source: "..\..\debug\a865r-debug.exe"; DestDir: "{app}\debug"; Flags: ignoreversion
Source: "..\bin\a865rctl.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
#endif
Source: "..\..\firmware\*"; DestDir: "{app}\firmware"; Flags: ignoreversion
Source: "..\..\tools\*.py"; DestDir: "{app}\tools"; Excludes: "test_*"; Flags: ignoreversion
Source: "..\..\docs\*.md"; DestDir: "{app}\docs"; Flags: ignoreversion
Source: "..\..\docs\*.json"; DestDir: "{app}\docs"; Flags: ignoreversion
Source: "..\..\debug\README.md"; DestDir: "{app}\debug"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\THIRD_PARTY.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSES.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "installed-mode.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "INSTALL-NOTES.txt"; DestDir: "{app}"; Flags: ignoreversion

Source: "..\..\images\*"; DestDir: "{app}\images"; Excludes: "__pycache__\*,*.pyc"; Flags: ignoreversion recursesubdirs createallsubdirs

#ifdef CrossPayload
Source: "{#CrossPayload}\adapter-update\*"; DestDir: "{app}\adapter-update"; Flags: ignoreversion recursesubdirs createallsubdirs
#else
Source: "..\adapter-update\*"; DestDir: "{app}\adapter-update"; Flags: ignoreversion recursesubdirs createallsubdirs
#endif
Source: "..\..\tools\Update-AverTV.ps1"; DestDir: "{app}\tools"; Flags: ignoreversion
Source: "..\..\tools\Adapter-UpdatePlan.ps1"; DestDir: "{app}\tools"; Flags: ignoreversion

[Icons]
Name: "{group}\Live TV!"; Filename: "{app}\player\live-tv.exe"; WorkingDir: "{app}"; Comment: "Live TV! {#AppVersion} - television playback and program guide"; IconFilename: "{app}\player\live-tv-ruby.ico"; AppUserModelID: "OpenVolarS.LiveTV"
Name: "{group}\A865R Debug Desk"; Filename: "{app}\debug\a865r-debug.exe"; WorkingDir: "{app}"; Comment: "Inspect the A865R tuner and export diagnostics"
Name: "{group}\Installation notes"; Filename: "{app}\INSTALL-NOTES.txt"
Name: "{group}\Uninstall Open Volar S"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Live TV!"; Filename: "{app}\player\live-tv.exe"; Tasks: desktopicon; IconFilename: "{app}\player\live-tv-ruby.ico"; AppUserModelID: "OpenVolarS.LiveTV"

[Run]
#ifdef CrossPayload
Filename: "{app}\bin\a865rctl.exe"; Parameters: "wine-setup ""{app}\wine\a865rctl"""; Flags: runhidden; Check: IsWine
#endif
Filename: "{sys}\regsvr32.exe"; Parameters: "/s /n /i:user ""{app}\adapter-update\x64\a865r_bda.dll"""; Flags: runhidden
Filename: "{syswow64}\regsvr32.exe"; Parameters: "/s /n /i:user ""{app}\adapter-update\x86\a865r_bda.dll"""; Flags: runhidden
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\tools\Update-AverTV.ps1"" -InstallDirectory ""{app}"""; Description: "Update the existing AverTV adapter (close AverTV first)"; Flags: postinstall skipifsilent runhidden; Check: NeedsAverTVUpdate

Filename: "{app}\player\live-tv.exe"; Description: "Run Live TV!"; WorkingDir: "{app}"; Flags: postinstall nowait skipifsilent runasoriginaluser

[UninstallRun]
#ifdef CrossPayload
Filename: "{app}\bin\a865rctl.exe"; Parameters: "wine-remove ""{app}\wine\a865rctl"""; Flags: runhidden; RunOnceId: "RemoveWineHelper"; Check: IsWine
#endif
Filename: "{sys}\regsvr32.exe"; Parameters: "/s /u /n /i:user ""{app}\adapter-update\x64\a865r_bda.dll"""; Flags: runhidden; RunOnceId: "UnregisterBDA64"
Filename: "{syswow64}\regsvr32.exe"; Parameters: "/s /u /n /i:user ""{app}\adapter-update\x86\a865r_bda.dll"""; Flags: runhidden; RunOnceId: "UnregisterBDA32"

[InstallDelete]
Type: files; Name: "{app}\debug\runtime\mpv.exe"
Type: files; Name: "{app}\debug\runtime\d3dcompiler_43.dll"
Type: files; Name: "{app}\debug\runtime\PROVENANCE.md"
Type: files; Name: "{app}\player\a865r-tv.exe"
; Replace the earlier generic name when upgrading in the same program group.
Type: files; Name: "{group}\Debug Desk.lnk"
Type: files; Name: "{group}\A865R TV.lnk"
Type: files; Name: "{autodesktop}\A865R TV.lnk"

[Code]
function IsWine: Boolean;
begin
  Result := RegKeyExists(HKCU, 'Software\Wine');
end;

function NeedsAverTVUpdate: Boolean;
var ExitCode: Integer;
begin
  Result := False;
  if not FileExists(ExpandConstant('{app}\tools\Update-AverTV.ps1')) then exit;
  if Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
    '-NoProfile -ExecutionPolicy Bypass -File "' + ExpandConstant('{app}\tools\Update-AverTV.ps1') +
    '" -InstallDirectory "' + ExpandConstant('{app}') + '" -CheckOnly',
    '', SW_HIDE, ewWaitUntilTerminated, ExitCode) then
    Result := ExitCode = 2;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  StartupCommand: String;
  ExpectedCommand: String;
begin
  if CurUninstallStep = usUninstall then begin
    ExpectedCommand := '"' + ExpandConstant('{app}\player\live-tv.exe') + '" --receiver-startup';
    if RegQueryStringValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'OpenVolarSReceiver', StartupCommand) then
      if CompareText(StartupCommand, ExpectedCommand) = 0 then
        RegDeleteValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'OpenVolarSReceiver');
  end;
end;