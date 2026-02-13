#define MyAppName "SwiftFind"

#ifndef AppVersion
  #define AppVersion "0.0.0-local"
#endif

#ifndef StageDir
  #error StageDir must be passed to ISCC via /DStageDir=...
#endif

#ifndef SetupIconPath
  #error SetupIconPath must be passed to ISCC via /DSetupIconPath=...
#endif

[Setup]
AppId={{E3A739E3-FAF7-4E18-BD8B-01744C9E7C27}
AppName={#MyAppName}
AppVersion={#AppVersion}
DefaultDirName={localappdata}\Programs\SwiftFind
DefaultGroupName=SwiftFind
OutputDir=artifacts\windows
OutputBaseFilename=swiftfind-{#AppVersion}-windows-x64-setup
Compression=lzma
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern
PrivilegesRequired=lowest
DisableDirPage=yes
DisableProgramGroupPage=yes
CloseApplications=yes
CloseApplicationsFilter=swiftfind-core.exe
RestartApplications=no
UninstallDisplayIcon={app}\bin\swiftfind-core.exe
SetupIconFile={#SetupIconPath}

[Files]
Source: "{#StageDir}\bin\swiftfind-core.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "{#StageDir}\assets\*"; DestDir: "{app}\assets"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#StageDir}\docs\*"; DestDir: "{app}\docs"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#StageDir}\scripts\*"; DestDir: "{app}\scripts"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\SwiftFind"; Filename: "{app}\bin\swiftfind-core.exe"; Parameters: "--background"
Name: "{autodesktop}\SwiftFind"; Filename: "{app}\bin\swiftfind-core.exe"; Parameters: "--background"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"

[Run]
Filename: "{app}\bin\swiftfind-core.exe"; Parameters: "--ensure-config"; Flags: runhidden
Filename: "{app}\bin\swiftfind-core.exe"; Parameters: "--sync-startup"; Flags: runhidden
Filename: "{app}\bin\swiftfind-core.exe"; Parameters: "--background"; Description: "Launch SwiftFind now"; Flags: runhidden nowait postinstall skipifsilent

[UninstallRun]
; Ask running instance to terminate cleanly first.
Filename: "{app}\bin\swiftfind-core.exe"; Parameters: "--quit"; Flags: runhidden waituntilterminated skipifdoesntexist; RunOnceId: "swiftfind-quit-runtime"
; Remove per-user startup registration even if config still had launch_at_startup=true.
Filename: "{cmd}"; Parameters: "/C reg delete HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v SwiftFind /f >NUL 2>&1 || exit /b 0"; Flags: runhidden; RunOnceId: "swiftfind-clear-startup"
; Hard-stop any leftover process to avoid ghost hotkey/runtime after uninstall.
Filename: "{cmd}"; Parameters: "/C taskkill /IM swiftfind-core.exe /F /T >NUL 2>&1 || exit /b 0"; Flags: runhidden; RunOnceId: "swiftfind-kill-runtime"
