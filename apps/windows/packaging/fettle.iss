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
Source: "{#NoticesDir}\LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#NoticesDir}\THIRD-PARTY-NOTICES.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Fettle"; Filename: "{app}\fettle.exe"
Name: "{autodesktop}\Fettle"; Filename: "{app}\fettle.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\fettle.exe"; Description: "{cm:LaunchProgram,Fettle}"; Flags: nowait postinstall skipifsilent

[Code]
// A cleared "Check for a new version" box turns the app's setting off. The app reads its settings
// field by field, so a first install writes just that field; an earlier install's settings keep
// everything else. A ticked box changes nothing, so it never undoes a choice made in the app.
procedure CurStepChanged(CurStep: TSetupStep);
var
  Path: String;
  Saved: AnsiString;
  Text: String;
begin
  if (CurStep = ssPostInstall) and not WizardIsTaskSelected('updatecheck') then
  begin
    Path := ExpandConstant('{localappdata}\Fettle\settings.json');
    Text := '';
    if LoadStringFromFile(Path, Saved) then
      Text := String(Saved);
    if Pos('"updates":true', Text) > 0 then
      StringChangeEx(Text, '"updates":true', '"updates":false', True)
    else if Pos('"updates":false', Text) = 0 then
      Text := '{"updates":false}';
    ForceDirectories(ExtractFileDir(Path));
    SaveStringToFile(Path, AnsiString(Text), False);
  end;
end;

// Uninstalling asks whether to remove Fettle's data too (it lives in the user's own AppData, so
// the installer never put it there). A silent uninstall keeps it.
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Data: String;
begin
  if CurUninstallStep = usPostUninstall then
  begin
    Data := ExpandConstant('{localappdata}\Fettle');
    if DirExists(Data) and not UninstallSilent then
      if MsgBox(CustomMessage('RemoveData'), mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES then
        DelTree(Data, True, True, True);
  end;
end;
