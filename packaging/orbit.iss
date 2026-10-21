#define AppName "Orbit"
#ifndef AppVersion
#define AppVersion "0.3.1"
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
function HasWebView2: Boolean;
var
  Version: String;
begin
  Result := (RegQueryStringValue(HKLM32, 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', Version) and (Version <> '') and (Version <> '0.0.0.0')) or
    (RegQueryStringValue(HKCU, 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', Version) and (Version <> '') and (Version <> '0.0.0.0'));
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Code: Integer;
  RuntimeInstaller: String;
begin
  Result := '';
  if HasWebView2 then Exit;
  RuntimeInstaller := ExpandConstant('{src}\MicrosoftEdgeWebView2RuntimeInstallerX64.exe');
  if not FileExists(RuntimeInstaller) then begin
    try
      DownloadTemporaryFile('https://go.microsoft.com/fwlink/p/?LinkId=2124703', 'MicrosoftEdgeWebview2Setup.exe', '', nil);
      RuntimeInstaller := ExpandConstant('{tmp}\MicrosoftEdgeWebview2Setup.exe');
    except
      Result := 'Orbit needs Microsoft Edge WebView2. Install the Evergreen x64 runtime, or place MicrosoftEdgeWebView2RuntimeInstallerX64.exe beside this installer for offline setup, then retry.';
      Exit;
    end;
  end;
  if not Exec(RuntimeInstaller, '/silent /install', '', SW_HIDE, ewWaitUntilTerminated, Code) then begin
    Result := 'WebView2 could not be installed. Install Microsoft Edge WebView2 and retry.';
    Exit;
  end;
  if not HasWebView2 then Result := 'WebView2 setup has not completed. Complete its installation and retry Orbit setup.';
end;

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
