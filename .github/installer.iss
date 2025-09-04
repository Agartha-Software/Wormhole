; Wormhole windows wizard Inno Setup

[Setup]
; NOTE: The value of AppId uniquely identifies this application. Do not use the same AppId value in installers for other applications.
; (To generate a new GUID, click Tools | Generate GUID inside the IDE.)
AppId={{627C9749-3020-4310-87C4-CF790AFC6DF0}
AppName=Wormhole
AppVersion=1.0
//AppVerName=Wormhole 1.0
AppPublisher=Agartha-Software
AppPublisherURL=https://github.com/Agartha-Software/Wormhole
AppSupportURL=https://github.com/Agartha-Software/Wormhole
AppUpdatesURL=https://github.com/Agartha-Software/Wormhole
SolidCompression=yes
WizardStyle=modern
PrivilegesRequiredOverridesAllowed=dialog
DefaultDirName={autopf}\Wormhole

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"

[Files]
Source: "..\target\release\wormhole.exe"; DestDir: "{app}"; Flags: allowunsafefiles
Source: "..\target\release\wormholed.exe"; DestDir: "{app}"; Flags: allowunsafefiles

[Tasks]
Name: "StartMenuEntry" ; Description: "Start wormhole service when Windows starts" ; GroupDescription: "Windows Startup"; MinVersion: 4,4;
Name: AddDefenderExclusion; Description: "Exclude Wormhole service from Windows Defender"; GroupDescription: "Security options"; Flags: unchecked

[icons]
Name: "{userstartup}\Wormhole Service"; Filename: "{app}\wormholed.exe"; Tasks: "StartMenuEntry"; Check: not IsAdminInstallMode
Name: "{commonstartup}\Wormhole Service"; Filename: "{app}\wormholed.exe"; Tasks: "StartMenuEntry"; Check: IsAdminInstallMode
;IconFilename: "{app}\icon.ico";

[Registry]
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"; \
    ValueType: expandsz; ValueName: "PATH"; ValueData: "{olddata};{app}"; \
    Check: NeedsAddPath('{app}')

[Run]
Filename: "powershell.exe"; \
  Parameters: "-ExecutionPolicy Bypass -NoProfile -WindowStyle Hidden -Command Add-MpPreference -ExclusionPath '""{app}\wormholed.exe""'"; \
  Flags: runhidden runascurrentuser; Tasks: AddDefenderExclusion
    
[Code]
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_LOCAL_MACHINE,
    'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
    'Path', OrigPath)
  then begin
    Result := True;
    exit;
  end;
  { look for the path with leading and trailing semicolon }
  { Pos() returns 0 if not found }
  Result := Pos(';' + Param + ';', ';' + OrigPath + ';') = 0;
end;