#define AppName "Orbit"
#ifndef AppVersion
#define AppVersion "0.1.1"
#endif
#define AppExeName "orbit.exe"

[Setup]
AppId={{0C47A2BE-0BB8-4833-B2F4-E180BDB29D9C}
AppName={#AppName}
AppVersion={#AppVersion}
AppMutex=Local\StillPomodoro.Desktop.v1
AppPublisher=Orbit contributors
DefaultDirName={localappdata}\Programs\Orbit
DefaultGroupName=Orbit
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.22621
OutputDir=..\dist
OutputBaseFilename=Orbit-{#AppVersion}-Setup-x64
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#AppExeName}
SetupIconFile=..\crates\app\assets\orbit.ico
LicenseFile=..\LICENSE
CloseApplications=yes
RestartApplications=no

[Files]
Source: "..\dist\Orbit-{#AppVersion}-windows-x64\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\Orbit"; Filename: "{app}\{#AppExeName}"; AppUserModelID: "Orbit.Pomodoro.Desktop"; AppUserModelToastActivatorCLSID: "1B59D91C-8435-40D2-8A63-A582210CA911"

[Registry]
Root: HKCU; Subkey: "Software\Classes\orbit-pomodoro"; ValueType: string; ValueData: "URL:Orbit"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\orbit-pomodoro"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""
Root: HKCU; Subkey: "Software\Classes\orbit-pomodoro\shell\open\command"; ValueType: string; ValueData: """{app}\{#AppExeName}"" --show"
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "Orbit"; Flags: uninsdeletevalue

[Run]
Filename: "{app}\{#AppExeName}"; Parameters: "--register-notifications"; Flags: runhidden waituntilterminated
Filename: "{app}\{#AppExeName}"; Description: "Open Orbit"; Flags: nowait postinstall skipifsilent

[InstallDelete]
Type: files; Name: "{app}\still-pomodoro.exe"
Type: files; Name: "{autoprograms}\Still Pomodoro.lnk"

[Code]
procedure CurStepChanged(CurStep: TSetupStep);
var
  PreviousCommand: String;
begin
  if CurStep = ssPostInstall then
  begin
    { Preserve a previous opt-in to startup when the executable is renamed. }
    if RegQueryStringValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'StillPomodoro', PreviousCommand) then
    begin
      if RegWriteStringValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'Orbit', '"' + ExpandConstant('{app}\orbit.exe') + '" --tray') then
        RegDeleteValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'StillPomodoro');
    end;
  end;
end;
