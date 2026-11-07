#ifndef AppVersion
  #define AppVersion "0.1.0-beta.2"
#endif

[Setup]
AppId={{8E6F4D52-9C7A-4B31-A2F5-2A4B7D9E6C10}
AppName=bbport-rust
AppVersion={#AppVersion}
AppPublisher=NotBhan
AppPublisherURL=https://github.com/NotBhan/bbport-rust
AppSupportURL=https://github.com/NotBhan/bbport-rust/issues
DefaultDirName={localappdata}\Programs\bbport-rust
DefaultGroupName=bbport-rust
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir=..\..\release
OutputBaseFilename=bbport-rust-{#AppVersion}-windows-x86_64-setup
SetupIconFile=..\..\launcher\bloodborne.ico
UninstallDisplayIcon={app}\bbport-gui.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64
ArchitecturesAllowed=x64
MinVersion=10.0

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"; Flags: unchecked

[Files]
Source: "..\..\dist\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs ignoreversion

[Icons]
Name: "{userprograms}\bbport-rust"; Filename: "{app}\bbport-gui.exe"; WorkingDir: "{app}"
Name: "{userprograms}\bbport-rust CLI"; Filename: "{app}\run-bbport-rs.cmd"; WorkingDir: "{app}"
Name: "{userdesktop}\bbport-rust"; Filename: "{app}\bbport-gui.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\bbport-gui.exe"; Description: "Launch bbport-rust"; Flags: nowait postinstall skipifsilent
