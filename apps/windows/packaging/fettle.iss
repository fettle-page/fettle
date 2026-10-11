; Fettle's installer (Inno Setup 6). Built by build.mjs, which passes the version, the exe's
; folder and where to write the installer:
;   npm run package:windows
; It also passes NoticesDir, the folder holding LICENSE.txt and THIRD-PARTY-NOTICES.txt.
; Installs for the current user by default (no administrator prompt); the first page offers
; "all users" instead, and /ALLUSERS or /CURRENTUSER choose on the command line. Silent installs
; (/VERYSILENT) work for IT departments. Upgrades install over the old version in place.

#ifndef AppVersion
  #error Pass /DAppVersion=x.y.z (build.mjs does)
#endif
#ifndef ExeDir
  #error Pass /DExeDir=<folder with fettle.exe> (build.mjs does)
#endif
#ifndef RuntimeDir
  #error Pass /DRuntimeDir=<folder with the Visual C++ runtime DLLs fettle.exe needs> (build.mjs does)
#endif
#ifndef NoticesDir
  #error Pass /DNoticesDir=<folder with LICENSE.txt and THIRD-PARTY-NOTICES.txt> (build.mjs does)
#endif
#ifndef OutDir
  #define OutDir "."
#endif

[Setup]
; The installer's identity: never change it, or upgrades become second installs.
AppId={{9404A274-E009-4582-B280-0BE4327BB7A1}
AppName=Fettle
AppVersion={#AppVersion}
AppVerName=Fettle {#AppVersion}
AppPublisher=Fettle
AppPublisherURL=https://fettle.page
AppSupportURL=https://fettle.page
AppUpdatesURL=https://fettle.page/download
VersionInfoVersion={#AppVersion}
VersionInfoProductName=Fettle
DefaultDirName={autopf}\Fettle
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog commandline
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
WizardStyle=modern
SetupIconFile=..\ui\img\fettle.ico
UninstallDisplayIcon={app}\fettle.exe
UninstallDisplayName=Fettle
; A running Fettle is closed (and not restarted) so its file can be replaced.
CloseApplications=yes
RestartApplications=no
Compression=lzma2/max
SolidCompression=yes
OutputDir={#OutDir}
OutputBaseFilename=Fettle-Setup-{#AppVersion}

[Languages]
; Before installing, a page shows what Fettle sends (only the update check) and links the privacy policy.
Name: "en"; MessagesFile: "compiler:Default.isl"; InfoBeforeFile: "privacy\privacy-en.txt"
Name: "it"; MessagesFile: "compiler:Languages\Italian.isl"; InfoBeforeFile: "privacy\privacy-it.txt"

[CustomMessages]
en.RemoveData=Also remove what Fettle keeps on this PC: its settings, the remembered check and the list of links you shared?%n%nThe links themselves stay online until they expire. Without that list, Fettle can no longer delete them for you.
it.RemoveData=Rimuovere anche ciò che Fettle conserva su questo PC: le impostazioni, l'ultimo controllo ricordato e l'elenco dei link condivisi?%n%nI link restano online fino alla scadenza. Senza quell'elenco, Fettle non potrà più eliminarli per te.
en.RemoveDataAll=Also remove what Fettle keeps for each account on this PC (found for %1): their settings, remembered checks and lists of shared links?%n%nThe links themselves stay online until they expire. Without those lists, Fettle can no longer delete them.
it.RemoveDataAll=Rimuovere anche ciò che Fettle conserva per ogni account di questo PC (trovati: %1): impostazioni, ultimi controlli ricordati ed elenchi dei link condivisi?%n%nI link restano online fino alla scadenza. Senza quegli elenchi, Fettle non potrà più eliminarli.
en.RemoveDataLeft=Fettle couldn't remove these folders (a file in one may be in use). You can delete them yourself:
it.RemoveDataLeft=Fettle non è riuscito a rimuovere queste cartelle (forse un file è in uso). Puoi eliminarle tu:
en.UpdatesGroup=Updates:
it.UpdatesGroup=Aggiornamenti:
en.UpdateCheck=Check for a new version (only the version number is asked for; you can change this later in Settings)
it.UpdateCheck=Controlla se c'è una nuova versione (chiede solo il numero di versione; puoi cambiarlo più avanti nelle Impostazioni)

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
; Ticked by default. IT departments can clear it in a silent install with /MERGETASKS="!updatecheck".
Name: "updatecheck"; Description: "{cm:UpdateCheck}"; GroupDescription: "{cm:UpdatesGroup}"

[Files]
Source: "{#ExeDir}\fettle.exe"; DestDir: "{app}"; Flags: ignoreversion
; The Visual C++ runtime fettle.exe loads from its own folder (see packaging/runtime.mjs).
Source: "{#RuntimeDir}\*.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#NoticesDir}\LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#NoticesDir}\THIRD-PARTY-NOTICES.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Fettle"; Filename: "{app}\fettle.exe"
Name: "{autodesktop}\Fettle"; Filename: "{app}\fettle.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\fettle.exe"; Description: "{cm:LaunchProgram,Fettle}"; Flags: nowait postinstall skipifsilent

[Registry]
; For all users, a cleared "Check for a new version" box is the default for every account on the PC:
; Fettle reads HKLM\Software\Fettle\UpdateCheck = 0 as "off" for anyone who hasn't chosen in Settings.
Root: HKLM; Subkey: "Software\Fettle"; ValueType: dword; ValueName: "UpdateCheck"; ValueData: 0; Tasks: not updatecheck; Check: IsAdminInstallMode; Flags: uninsdeletevalue uninsdeletekeyifempty

[Code]
// A cleared "Check for a new version" box turns the app's setting off, for a user who hasn't chosen
// yet: no settings, or settings without "updates" (the field is added, the rest kept). A choice made
// in Settings survives every upgrade, silent ones and restored task boxes included. An all-users
// install leaves per-user settings alone: they'd land in the installing account's profile (or
// SYSTEM's, under deployment tools); the machine-wide value in [Registry] covers every account.
procedure CurStepChanged(CurStep: TSetupStep);
var
  Path: String;
  Saved: AnsiString;
  Text: String;
begin
  // Installing again for all users with the box ticked takes back an earlier machine-wide "off".
  if (CurStep = ssPostInstall) and IsAdminInstallMode and WizardIsTaskSelected('updatecheck') then
    RegDeleteValue(HKLM, 'Software\Fettle', 'UpdateCheck');
  if (CurStep = ssPostInstall) and not IsAdminInstallMode and not WizardIsTaskSelected('updatecheck') then
  begin
    Path := ExpandConstant('{localappdata}\Fettle\settings.json');
    Text := '';
    if LoadStringFromFile(Path, Saved) then
      Text := String(Saved);
    if Pos('"updates"', Text) = 0 then
    begin
      if (Pos('{', Text) = 0) or (Pos('{}', Text) > 0) then
        Text := '{"updates":false}'
      else
        Insert('"updates":false,', Text, Pos('{', Text) + 1);
      ForceDirectories(ExtractFileDir(Path));
      SaveStringToFile(Path, AnsiString(Text), False);
    end;
  end;
end;

// Every account's Fettle folder that exists, from Windows' list of user profiles (local and
// Microsoft Entra accounts). An all-users uninstall runs as administrator, so it can reach them all.
function AccountDataFolders: TArrayOfString;
var
  Sids: TArrayOfString;
  I, Count: Integer;
  Profile, Folder: String;
begin
  Count := 0;
  SetArrayLength(Result, 0);
  if not RegGetSubkeyNames(HKLM, 'SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList', Sids) then
    Exit;
  for I := 0 to GetArrayLength(Sids) - 1 do
    if ((Pos('S-1-5-21-', Sids[I]) = 1) or (Pos('S-1-12-1-', Sids[I]) = 1)) and
       RegQueryStringValue(HKLM, 'SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList\' + Sids[I], 'ProfileImagePath', Profile) then
    begin
      StringChangeEx(Profile, '%SystemDrive%', GetEnv('SystemDrive'), True);
      Folder := AddBackslash(Profile) + 'AppData\Local\Fettle';
      if DirExists(Folder) then
      begin
        Count := Count + 1;
        SetArrayLength(Result, Count);
        Result[Count - 1] := Folder;
      end;
    end;
end;

// Uninstalling asks whether to remove Fettle's data too (it lives in each user's own AppData, so the
// installer never put it there). For one user, that user's folder; for all users, every account's,
// naming any folder that couldn't be removed. A silent uninstall keeps it all.
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Data, Left: String;
  Folders: TArrayOfString;
  I: Integer;
begin
  if (CurUninstallStep <> usPostUninstall) or UninstallSilent then
    Exit;
  if IsAdminInstallMode then
  begin
    Folders := AccountDataFolders;
    if GetArrayLength(Folders) = 0 then
      Exit;
    if MsgBox(FmtMessage(CustomMessage('RemoveDataAll'), [IntToStr(GetArrayLength(Folders))]), mbConfirmation, MB_YESNO or MB_DEFBUTTON2) <> IDYES then
      Exit;
    Left := '';
    for I := 0 to GetArrayLength(Folders) - 1 do
    begin
      DelTree(Folders[I], True, True, True);
      if DirExists(Folders[I]) then
        Left := Left + #13#10 + Folders[I];
    end;
    if Left <> '' then
      MsgBox(CustomMessage('RemoveDataLeft') + #13#10 + Left, mbInformation, MB_OK);
  end
  else
  begin
    Data := ExpandConstant('{localappdata}\Fettle');
    if DirExists(Data) then
      if MsgBox(CustomMessage('RemoveData'), mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES then
        DelTree(Data, True, True, True);
  end;
end;
