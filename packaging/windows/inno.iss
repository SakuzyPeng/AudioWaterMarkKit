; AWMKit Windows Installer (Inno Setup) - Draft for WinUI phase
; Build example:
; iscc /DAppVersion=0.1.3 /DAppSourceDir="C:\\path\\to\\app" /DRustCliSource="C:\\path\\to\\awmkit.exe" /DOutputDir="C:\\path\\to\\dist" packaging\\windows\\inno.iss

#ifndef AppName
  #define AppName "AWMKit"
#endif

#ifndef AppVersion
  #define AppVersion "0.1.3"
#endif

#ifndef AppPublisher
  #define AppPublisher "AWMKit"
#endif

#ifndef AppExeName
  #define AppExeName "AWMKit.exe"
#endif

#ifndef AppSourceDir
  #define AppSourceDir "staging\\app"
#endif

#ifndef RustCliSource
  #define RustCliSource "staging\\app\\bin\\awmkit.exe"
#endif

#ifndef OutputDir
  #define OutputDir "dist"
#endif

[Setup]
AppId={{E2F714D7-5A0D-4B10-9C48-8EDC5B8DB6FA}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={autopf}\\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
OutputDir={#OutputDir}
OutputBaseFilename=AWMKit-Setup-{#AppVersion}
Compression=lzma
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesInstallIn64BitMode=x64

[Languages]
Name: "chinesesimp"; MessagesFile: "compiler:Default.isl"
Name: "english"; MessagesFile: "compiler:Languages\\English.isl"

[Files]
Source: "{#AppSourceDir}\\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#RustCliSource}"; DestDir: "{app}\\bin"; Flags: ignoreversion skipifsourcedoesntexist

[Icons]
Name: "{autoprograms}\\{#AppName}"; Filename: "{app}\\{#AppExeName}"
Name: "{autodesktop}\\{#AppName}"; Filename: "{app}\\{#AppExeName}"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; GroupDescription: "附加任务:"; Flags: unchecked

[Run]
Filename: "{app}\\{#AppExeName}"; Description: "启动 {#AppName}"; Flags: nowait postinstall skipifsilent

[Code]
function AWMKitUserDataDir: string;
begin
  Result := ExpandConstant('{userprofile}\\.awmkit');
end;

procedure TryDeleteUserData;
var
  DataDir: string;
begin
  DataDir := AWMKitUserDataDir;
  if DirExists(DataDir) then
  begin
    if MsgBox('检测到用户数据目录：' + #13#10 + DataDir + #13#10#13#10 + '是否同时清理数据库与缓存？', mbConfirmation, MB_YESNO) = IDYES then
    begin
      DelTree(DataDir, True, True, True);
    end;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
  begin
    TryDeleteUserData;
  end;
end;
