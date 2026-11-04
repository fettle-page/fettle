//! What Fettle keeps on this PC, in `%LOCALAPPDATA%\Fettle\`: the settings, the last ten checks (the
//! same redacted reports the receipts show, with the client and notes typed for each), the logo for
//! printouts and the share links it can still delete. A file that is
//! missing, broken, of the wrong shape or from a newer Fettle is ignored and replaced: it can never
//! stop the app from opening.

use crate::state::Settings;
use fettle_facts::{Report, SCHEMA_VERSION};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// What this PC keeps about a link it shared, so it can delete it later. The token is never shown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedLink {
    pub id: String,
    pub url: String,
    pub delete_token: String,
    pub expires_at: String,
}

/// How many checks this PC keeps: an 11th drops the oldest.
pub const MAX_CHECKS: usize = 10;

/// A check this PC keeps: its report, and the client and notes typed for its printout (only ever
/// on paper, never in the report, a share or a saved JSON file).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedCheck {
    pub report: Report,
    #[serde(default)]
    pub client: String,
    #[serde(default)]
    pub notes: String,
}

impl SavedCheck {
    pub fn new(report: Report) -> SavedCheck {
        SavedCheck { report, client: String::new(), notes: String::new() }
    }

    /// A check is known by when it was made.
    pub fn id(&self) -> &str {
        &self.report.collected_at
    }

    /// When it was made, in ms since 1970 (None: unreadable).
    pub fn at(&self) -> Option<i64> {
        fettle_verdicts::fmt::parse_rfc3339(&self.report.collected_at)
    }
}

/// The saved checks as they're kept: each once (the first entry of an id wins), newest first, ten at most.
pub fn tidy_checks(checks: Vec<SavedCheck>) -> Vec<SavedCheck> {
    let mut kept: Vec<SavedCheck> = Vec::with_capacity(checks.len());
    for c in checks {
        if !kept.iter().any(|k| k.id() == c.id()) {
            kept.push(c);
        }
    }
    // Stable, so two of the same moment keep their order.
    kept.sort_by_key(|c| std::cmp::Reverse(c.at().unwrap_or(i64::MIN)));
    kept.truncate(MAX_CHECKS);
    kept
}

pub trait Store {
    fn load_settings(&self) -> Settings;
    fn save_settings(&self, settings: &Settings);
    /// The saved checks, newest first.
    fn load_checks(&self) -> Vec<SavedCheck>;
    fn save_checks(&self, checks: &[SavedCheck]);
    /// Forgets every saved check.
    fn clear_checks(&self);
    /// Runs `f` while no other Fettle window can change the saved checks (best effort).
    fn with_checks_locked(&self, f: &mut dyn FnMut()) {
        f()
    }
    /// Adds a check (replacing a saved one of the same id). Read again first, as `add_link` does,
    /// so another window's checks stay.
    fn add_check(&self, check: &SavedCheck) {
        self.with_checks_locked(&mut || {
            let mut checks = vec![check.clone()];
            checks.extend(self.load_checks());
            self.save_checks(&tidy_checks(checks));
        });
    }
    /// Forgets one check, keeping every other window's.
    fn delete_check(&self, id: &str) {
        self.with_checks_locked(&mut || {
            let checks = self.load_checks();
            let kept: Vec<SavedCheck> = checks.iter().filter(|c| c.id() != id).cloned().collect();
            if kept.len() != checks.len() {
                self.save_checks(&kept);
            }
        });
    }
    /// The client and notes for one saved check's printout.
    fn set_check_notes(&self, id: &str, client: &str, notes: &str) {
        self.with_checks_locked(&mut || {
            let mut checks = self.load_checks();
            if let Some(c) = checks.iter_mut().find(|c| c.id() == id) {
                c.client = client.to_string();
                c.notes = notes.to_string();
                self.save_checks(&checks);
            }
        });
    }
    /// The logo for printouts, as the PNG saved (None: there's none).
    fn load_logo(&self) -> Option<Vec<u8>>;
    /// Saves the logo, already prepared as a PNG.
    fn save_logo(&self, png: &[u8]);
    fn clear_logo(&self);
    /// The links still alive at `now_ms`; expired ones are forgotten.
    fn load_links(&self, now_ms: i64) -> Vec<SavedLink>;
    fn save_links(&self, links: &[SavedLink]);
    /// Runs `f` while no other Fettle window can change the saved links (best effort).
    fn with_links_locked(&self, f: &mut dyn FnMut()) {
        f()
    }
    /// Adds a link to the ones saved now. Another window may have saved its own since this one
    /// read the list, so the list is read again first: writing this window's copy whole would drop them.
    fn add_link(&self, link: &SavedLink, now_ms: i64) {
        self.with_links_locked(&mut || {
            let mut links = self.load_links(now_ms);
            if !links.iter().any(|l| l.id == link.id) {
                links.push(link.clone());
            }
            self.save_links(&links);
        });
    }
    /// Forgets these links (deleted, or gone from the website), keeping every other window's.
    fn forget_links(&self, ids: &[String], now_ms: i64) {
        self.with_links_locked(&mut || {
            let links = self.load_links(now_ms);
            let kept: Vec<SavedLink> = links.iter().filter(|l| !ids.contains(&l.id)).cloned().collect();
            if kept.len() != links.len() {
                self.save_links(&kept);
            }
        });
    }
    /// The last update check (a missing or bad file is "never asked").
    fn load_update(&self) -> crate::update::UpdateRecord;
    fn save_update(&self, record: &crate::update::UpdateRecord);
}

/// The real store: JSON files in one folder.
pub struct FileStore {
    dir: PathBuf,
    /// The machine-wide update-check default an all-users install can set (None: not set).
    machine_updates: Option<bool>,
}

impl FileStore {
    pub fn new(dir: PathBuf) -> FileStore {
        FileStore { dir, machine_updates: None }
    }

    /// The update check's default for a user who hasn't chosen in Settings: off when an all-users
    /// install had its box cleared (`HKLM\Software\Fettle\UpdateCheck` = 0). A user's own choice wins.
    pub fn with_machine_updates(self, machine_updates: Option<bool>) -> FileStore {
        FileStore { machine_updates, ..self }
    }

    /// `%LOCALAPPDATA%\Fettle`, or the portable copy's `Fettle data` folder beside the exe.
    pub fn default_dir() -> PathBuf {
        let exe = std::env::current_exe().ok();
        data_dir(exe.as_deref().and_then(Path::parent), std::env::var_os("LOCALAPPDATA"))
    }
}

/// The portable copy's folder: when it sits beside fettle.exe, everything Fettle keeps goes there,
/// so a copy on a USB stick leaves nothing on the PC it runs on.
pub const PORTABLE_DIR: &str = "Fettle data";

/// Where Fettle keeps its files, given the exe's folder and `%LOCALAPPDATA%`.
pub fn data_dir(exe_dir: Option<&Path>, local_app_data: Option<std::ffi::OsString>) -> PathBuf {
    match exe_dir.map(|d| d.join(PORTABLE_DIR)) {
        Some(portable) if portable.is_dir() => portable,
        _ => PathBuf::from(local_app_data.unwrap_or_default()).join("Fettle"),
    }
}

const SETTINGS: &str = "settings.json";
/// Where Fettle 1.0 kept its one remembered check: moved into checks.json, then deleted.
const LAST: &str = "last-report.json";
const CHECKS: &str = "checks.json";
/// The shape of checks.json; a newer one is from a newer Fettle and is ignored.
const CHECKS_VERSION: u64 = 1;
const LOGO: &str = "logo.png";
const LINKS: &str = "links.json";
const UPDATE: &str = "update.json";
const LINKS_LOCK: &str = "links.lock";
const CHECKS_LOCK: &str = "checks.lock";

impl FileStore {
    fn read(&self, name: &str) -> Option<serde_json::Value> {
        serde_json::from_str(&std::fs::read_to_string(self.dir.join(name)).ok()?).ok()
    }

    /// Best effort: a full disk or a locked file just means it isn't kept this time (false).
    fn write(&self, name: &str, value: &impl Serialize) -> bool {
        let Ok(text) = serde_json::to_string(value) else { return false };
        self.write_bytes(name, text.as_bytes())
    }

    fn write_bytes(&self, name: &str, bytes: &[u8]) -> bool {
        let _ = std::fs::create_dir_all(&self.dir);
        // Written aside, then moved over the old file, so a crash never leaves half a file. The
        // temporary file is this process's own: another Fettle window may be saving at the same moment.
        let tmp = self.dir.join(format!("{name}.{}.tmp", std::process::id()));
        if std::fs::write(&tmp, bytes).is_err() || std::fs::rename(&tmp, self.dir.join(name)).is_err() {
            let _ = std::fs::remove_file(&tmp);
            return false;
        }
        true
    }

    /// Fettle 1.0's remembered check, if there is one, becomes the first saved check. The old file
    /// goes once checks.json holds it, or at once when it can't be used.
    fn move_last_check(&self) -> Vec<SavedCheck> {
        if !self.dir.join(LAST).exists() {
            return Vec::new();
        }
        let checks: Vec<SavedCheck> = self.read(LAST).and_then(usable_report).map(SavedCheck::new).into_iter().collect();
        if checks.is_empty() || self.write(CHECKS, &checks_file(&checks)) {
            self.remove(LAST);
        }
        checks
    }

    /// Holds `name` open with no sharing (and gone when closed), waiting up to two seconds for it.
    fn lock(&self, name: &str) -> Option<std::fs::File> {
        let _ = std::fs::create_dir_all(&self.dir);
        for _ in 0..40 {
            let mut options = std::fs::OpenOptions::new();
            options.read(true).write(true).create(true);
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                const DELETE: u32 = 0x0001_0000;
                const GENERIC_READ_WRITE: u32 = 0x8000_0000 | 0x4000_0000;
                const FILE_FLAG_DELETE_ON_CLOSE: u32 = 0x0400_0000;
                options.share_mode(0).access_mode(GENERIC_READ_WRITE | DELETE).custom_flags(FILE_FLAG_DELETE_ON_CLOSE);
            }
            if let Ok(file) = options.open(self.dir.join(name)) {
                return Some(file);
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        None
    }

    fn remove(&self, name: &str) {
        let _ = std::fs::remove_file(self.dir.join(name));
    }
}

/// A report this version can turn into a receipt: not from a newer Fettle, its time readable.
fn usable_report(value: serde_json::Value) -> Option<Report> {
    if !value["schema_version"].as_u64().is_some_and(|n| n <= u64::from(SCHEMA_VERSION)) {
        return None;
    }
    let report = serde_json::from_value::<Report>(value).ok()?;
    fettle_receipt::receipt_for(&report, fettle_verdicts::Locale::En, fettle_receipt::Zone::Utc).is_ok().then_some(report)
}

/// One saved check as stored, or None when its report can't be used.
fn usable_check(value: &serde_json::Value) -> Option<SavedCheck> {
    let report = usable_report(value.get("report")?.clone())?;
    let text = |name: &str| value.get(name).and_then(serde_json::Value::as_str).unwrap_or_default().to_string();
    Some(SavedCheck { report, client: text("client"), notes: text("notes") })
}

fn checks_file(checks: &[SavedCheck]) -> serde_json::Value {
    serde_json::json!({ "version": CHECKS_VERSION, "checks": checks })
}

/// One field of a JSON object, or the default when it's missing or of the wrong type.
fn field<T: serde::de::DeserializeOwned>(object: &serde_json::Value, name: &str, default: T) -> T {
    object.get(name).and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or(default)
}

impl Store for FileStore {
    fn load_settings(&self) -> Settings {
        let d = Settings { updates: self.machine_updates.unwrap_or(true), ..Settings::default() };
        let Some(o) = self.read(SETTINGS).filter(serde_json::Value::is_object) else { return d };
        Settings {
            remember: field(&o, "remember", d.remember),
            updates: field(&o, "updates", d.updates),
            update_every: field(&o, "update_every", d.update_every),
            show_private: field(&o, "show_private", d.show_private),
            lang: field(&o, "lang", d.lang),
            theme: field(&o, "theme", d.theme),
            technician: field(&o, "technician", d.technician),
            updates_chosen: o.get("updates").is_some_and(serde_json::Value::is_boolean),
        }
    }

    /// Saved whole, except "updates" until the user has chosen: changing only the theme mustn't
    /// turn the PC's default into this user's choice.
    fn save_settings(&self, settings: &Settings) {
        let Ok(mut value) = serde_json::to_value(settings) else { return };
        if !settings.updates_chosen
            && let Some(o) = value.as_object_mut()
        {
            o.remove("updates");
        }
        self.write(SETTINGS, &value);
    }

    /// A broken file, or one from a newer Fettle, is no checks; an entry this version can't use
    /// is left out and the others kept.
    fn load_checks(&self) -> Vec<SavedCheck> {
        if !self.dir.join(CHECKS).exists() {
            return self.move_last_check();
        }
        let Some(file) = self.read(CHECKS).filter(|v| v["version"].as_u64().is_some_and(|n| (1..=CHECKS_VERSION).contains(&n))) else { return Vec::new() };
        let entries = file["checks"].as_array().cloned().unwrap_or_default();
        tidy_checks(entries.iter().filter_map(usable_check).collect())
    }

    fn save_checks(&self, checks: &[SavedCheck]) {
        self.write(CHECKS, &checks_file(checks));
    }

    fn clear_checks(&self) {
        self.remove(CHECKS);
        self.remove(LAST);
    }

    /// checks.lock, held like links.lock (gone when it's let go).
    fn with_checks_locked(&self, f: &mut dyn FnMut()) {
        let _lock = self.lock(CHECKS_LOCK);
        f()
    }

    fn load_logo(&self) -> Option<Vec<u8>> {
        std::fs::read(self.dir.join(LOGO)).ok().filter(|b| !b.is_empty())
    }

    fn save_logo(&self, png: &[u8]) {
        self.write_bytes(LOGO, png);
    }

    fn clear_logo(&self) {
        self.remove(LOGO);
    }

    fn load_links(&self, now_ms: i64) -> Vec<SavedLink> {
        let stored = self.read(LINKS).and_then(|v| v.as_array().cloned()).unwrap_or_default();
        let all: Vec<SavedLink> = stored.iter().filter_map(|v| serde_json::from_value(v.clone()).ok()).collect();
        // The Worker has already stopped serving expired links.
        let alive: Vec<SavedLink> = all.iter().filter(|l| fettle_verdicts::fmt::parse_rfc3339(&l.expires_at).is_some_and(|t| t > now_ms)).cloned().collect();
        if alive.len() != stored.len() && self.dir.join(LINKS).exists() {
            self.save_links(&alive);
        }
        alive
    }

    fn load_update(&self) -> crate::update::UpdateRecord {
        self.read(UPDATE).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default()
    }

    fn save_update(&self, record: &crate::update::UpdateRecord) {
        self.write(UPDATE, record);
    }

    fn save_links(&self, links: &[SavedLink]) {
        self.write(LINKS, &links);
    }

    /// The lock is links.lock, open with no sharing, so another window's open fails until this one
    /// lets go; Windows deletes it when it's closed, even if Fettle stops halfway. A window that can't
    /// get it within two seconds goes ahead anyway rather than lose the change.
    fn with_links_locked(&self, f: &mut dyn FnMut()) {
        let _lock = self.lock(LINKS_LOCK);
        f()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::state::ThemeChoice;
    use fettle_verdicts::Locale;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A test's scratch folder under target/tmp (never Windows' Temp). It goes when the last handle
    /// to it does, which is also when its test fails or panics, so a run leaves nothing behind.
    #[derive(Clone)]
    pub(crate) struct Scratch(std::sync::Arc<ScratchDir>);

    pub(crate) struct ScratchDir(PathBuf);

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            remove_scratch(&self.0);
        }
    }

    /// Removes a test's folder, or the file a test put in its place. remove_dir_all removes a
    /// junction inside without following it. The shared folder above stays: removing it while
    /// another test is creating its own inside would fail that test.
    pub(crate) fn remove_scratch(path: &Path) {
        match std::fs::symlink_metadata(path) {
            Ok(m) if m.is_dir() => drop(std::fs::remove_dir_all(path)),
            Ok(_) => drop(std::fs::remove_file(path)),
            Err(_) => {}
        }
    }

    impl AsRef<Path> for Scratch {
        fn as_ref(&self) -> &Path {
            self
        }
    }

    impl std::ops::Deref for Scratch {
        type Target = Path;
        fn deref(&self) -> &Path {
            &self.0.0
        }
    }

    /// A fresh, empty, not yet created folder for one test (`target/tmp/fettle-unit/<n>`).
    pub(crate) fn temp_dir() -> Scratch {
        static N: AtomicUsize = AtomicUsize::new(0);
        scratch_at(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/fettle-unit").join(N.fetch_add(1, Ordering::Relaxed).to_string()))
    }

    /// A scratch folder at `path`, emptied first (a run killed before its clean-up leaves one).
    pub(crate) fn scratch_at(path: PathBuf) -> Scratch {
        remove_scratch(&path);
        Scratch(std::sync::Arc::new(ScratchDir(path)))
    }

    #[test]
    fn a_scratch_path_a_test_made_into_a_file_goes_too() {
        // Some tests put a file where the data folder should be (a folder that can't be written).
        let dir = temp_dir();
        let path = dir.to_path_buf();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "not a folder").unwrap();
        drop(dir);
        assert!(!path.exists(), "{path:?} was left behind");
        // And a new scratch folder at a path a killed run left as a file starts empty.
        std::fs::write(&path, "left from a killed run").unwrap();
        let again = scratch_at(path.clone());
        assert!(!again.exists());
    }

    #[test]
    fn a_tests_scratch_folder_goes_even_when_the_test_panics() {
        let dir = temp_dir();
        let path = dir.to_path_buf();
        let result = std::panic::catch_unwind(move || {
            std::fs::create_dir_all(dir.join("inner")).unwrap();
            std::fs::write(dir.join("inner").join("file.json"), "{}").unwrap();
            panic!("a failing test");
        });
        assert!(result.is_err());
        assert!(!path.exists(), "{path:?} was left behind");
    }

    pub(crate) fn dump_report() -> Report {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/fettle-receipt/tests/parity/cases");
        let file = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).find(|p| p.file_name().unwrap().to_string_lossy().starts_with("dump-dev-workstation-user")).unwrap();
        let case: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
        serde_json::from_value(case["report"].clone()).unwrap()
    }

    #[test]
    fn a_fettle_data_folder_beside_the_exe_makes_it_portable() {
        let app = temp_dir();
        std::fs::create_dir_all(&app).unwrap();
        let local = std::ffi::OsString::from(r"C:\Users\someone\AppData\Local");
        // Installed: no such folder, so the user's AppData.
        assert_eq!(data_dir(Some(&app), Some(local.clone())), PathBuf::from(r"C:\Users\someone\AppData\Local").join("Fettle"));
        // Portable: the folder beside the exe, whatever AppData says.
        std::fs::create_dir_all(app.join(PORTABLE_DIR)).unwrap();
        assert_eq!(data_dir(Some(&app), Some(local)), app.join(PORTABLE_DIR));
        // A file of that name isn't the folder.
        let other = temp_dir();
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join(PORTABLE_DIR), "").unwrap();
        assert_eq!(data_dir(Some(&other), None), PathBuf::from("Fettle"));
    }

    #[test]
    fn a_machine_wide_update_setting_is_the_default_until_the_user_chooses() {
        // An all-users install with the box cleared: every account starts with the check off.
        let dir = temp_dir();
        let store = FileStore::new(dir.to_path_buf()).with_machine_updates(Some(false));
        assert!(!store.load_settings().updates, "no settings yet");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("settings.json"), r#"{"theme":"dark"}"#).unwrap();
        assert!(!store.load_settings().updates, "settings without a choice about updates");
        // A choice the user made in Settings wins: it's their own account.
        std::fs::write(dir.join("settings.json"), r#"{"updates":true}"#).unwrap();
        assert!(store.load_settings().updates);
        // Without a machine-wide setting, the check is on unless the user turned it off.
        let dir = temp_dir();
        assert!(FileStore::new(dir.to_path_buf()).with_machine_updates(None).load_settings().updates);
    }

    #[test]
    fn updates_are_saved_only_once_the_user_chooses() {
        // An all-users install turned the check off for the PC; this user changes only the theme.
        let dir = temp_dir();
        let store = FileStore::new(dir.to_path_buf()).with_machine_updates(Some(false));
        let mut s = store.load_settings();
        s.theme = ThemeChoice::Dark;
        store.save_settings(&s);
        let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("settings.json")).unwrap()).unwrap();
        assert!(saved.get("updates").is_none(), "no choice made, none saved: {saved}");
        // So the machine-wide default still decides, whatever it becomes later.
        assert!(FileStore::new(dir.to_path_buf()).with_machine_updates(None).load_settings().updates);
        // Once the user chooses in Settings, it's theirs and saved.
        s.updates = false;
        s.updates_chosen = true;
        store.save_settings(&s);
        assert!(!FileStore::new(dir.to_path_buf()).with_machine_updates(None).load_settings().updates);
        assert!(FileStore::new(dir.to_path_buf()).load_settings().updates_chosen);
    }

    #[test]
    fn settings_round_trip_and_start_from_defaults() {
        let dir = temp_dir();
        let store = FileStore::new(dir.to_path_buf());
        assert_eq!(store.load_settings(), Settings::default());
        let s = Settings { remember: false, show_private: true, lang: Some(Locale::It), theme: ThemeChoice::Oled, ..Settings::default() };
        store.save_settings(&s);
        assert_eq!(store.load_settings(), s);
    }

    #[test]
    fn broken_settings_fall_back_to_defaults_field_by_field() {
        let dir = temp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("settings.json"), r#"{"remember": "yes", "theme": "dark", "lang": 7}"#).unwrap();
        let s = FileStore::new(dir.to_path_buf()).load_settings();
        assert_eq!((s.remember, s.theme, s.lang), (true, ThemeChoice::Dark, None));
        std::fs::write(dir.join("settings.json"), "{not json").unwrap();
        assert_eq!(FileStore::new(dir.to_path_buf()).load_settings(), Settings::default());
    }

    // The installer's "check for updates" box (fettle.iss): cleared, it writes {"updates":false} on a
    // first install, or swaps "updates":true for false in the settings an older install saved.
    #[test]
    fn the_installer_can_turn_the_update_check_off() {
        let dir = temp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("settings.json"), r#"{"updates":false}"#).unwrap();
        assert_eq!(FileStore::new(dir.to_path_buf()).load_settings(), Settings { updates: false, updates_chosen: true, ..Settings::default() });

        // Settings saved without a choice about updates have no "updates" field, so the installer
        // (which only writes when there's none) adds one right after the "{", keeping the rest.
        let saved = Settings { remember: false, theme: ThemeChoice::Oled, ..Settings::default() };
        let store = FileStore::new(dir.to_path_buf());
        store.save_settings(&saved);
        let text = std::fs::read_to_string(dir.join("settings.json")).unwrap();
        assert!(!text.contains(r#""updates""#), "no choice made, none saved: {text}");
        assert!(text.starts_with('{'), "the installer adds its field after the opening brace: {text}");
        std::fs::write(dir.join("settings.json"), text.replacen('{', r#"{"updates":false,"#, 1)).unwrap();
        assert_eq!(store.load_settings(), Settings { updates: false, updates_chosen: true, ..saved });
    }

    fn check_at(at: &str) -> SavedCheck {
        SavedCheck { report: Report { collected_at: at.into(), ..dump_report() }, client: String::new(), notes: String::new() }
    }

    fn ids(store: &FileStore) -> Vec<String> {
        store.load_checks().into_iter().map(|c| c.id().to_string()).collect()
    }

    #[test]
    fn the_last_ten_checks_are_kept_newest_first_and_never_twice() {
        let dir = temp_dir();
        let store = FileStore::new(dir.to_path_buf());
        assert!(store.load_checks().is_empty());
        for day in 1..=12 {
            store.add_check(&check_at(&format!("2026-10-{day:02}T10:00:00Z")));
        }
        let kept = ids(&store);
        assert_eq!(kept.len(), 10, "{kept:?}");
        assert_eq!(kept.first().map(String::as_str), Some("2026-10-12T10:00:00Z"));
        assert_eq!(kept.last().map(String::as_str), Some("2026-10-03T10:00:00Z"), "the two oldest went");
        // The same check again replaces its entry rather than adding a second one.
        store.add_check(&SavedCheck { client: "Rossi".into(), ..check_at("2026-10-12T10:00:00Z") });
        assert_eq!(ids(&store), kept);
        assert_eq!(store.load_checks()[0].client, "Rossi");
        // An older check added late still goes in its place by date.
        store.add_check(&check_at("2026-10-05T12:00:00Z"));
        let now = ids(&store);
        assert_eq!((now[7].as_str(), now[9].as_str()), ("2026-10-05T12:00:00Z", "2026-10-04T10:00:00Z"));
    }

    #[test]
    fn the_last_check_of_an_older_fettle_becomes_the_first_saved_check() {
        let dir = temp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let r = dump_report();
        std::fs::write(dir.join("last-report.json"), serde_json::to_string(&r).unwrap()).unwrap();
        let store = FileStore::new(dir.to_path_buf());
        assert_eq!(store.load_checks(), vec![SavedCheck { report: r, client: String::new(), notes: String::new() }]);
        assert!(dir.join("checks.json").exists());
        assert!(!dir.join("last-report.json").exists(), "moved, not copied");
        assert_eq!(store.load_checks().len(), 1);
        // One it can't use is just removed.
        std::fs::remove_file(dir.join("checks.json")).unwrap();
        std::fs::write(dir.join("last-report.json"), "[1, 2").unwrap();
        assert!(store.load_checks().is_empty());
        assert!(!dir.join("last-report.json").exists());
    }

    #[test]
    fn a_broken_or_newer_checks_file_is_ignored() {
        let dir = temp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let store = FileStore::new(dir.to_path_buf());
        std::fs::write(dir.join("checks.json"), "[1, 2").unwrap();
        assert!(store.load_checks().is_empty());
        let good = serde_json::to_value(check_at("2026-10-09T10:00:00Z")).unwrap();
        std::fs::write(dir.join("checks.json"), serde_json::json!({ "version": 2, "checks": [good] }).to_string()).unwrap();
        assert!(store.load_checks().is_empty(), "from a newer Fettle");
        // One bad entry (newer report, unreadable time, wrong shape) goes; the others stay.
        let mut newer = good.clone();
        newer["report"]["schema_version"] = (SCHEMA_VERSION + 1).into();
        let mut odd = good.clone();
        odd["report"]["collected_at"] = "2026-02-32T10:00:00Z".into();
        let file = serde_json::json!({ "version": 1, "checks": [newer, odd, { "report": 7 }, good] });
        std::fs::write(dir.join("checks.json"), file.to_string()).unwrap();
        assert_eq!(ids(&store), ["2026-10-09T10:00:00Z"]);
        // Old entries without client or notes load with them empty.
        let file = serde_json::json!({ "version": 1, "checks": [{ "report": good["report"] }] });
        std::fs::write(dir.join("checks.json"), file.to_string()).unwrap();
        assert_eq!(store.load_checks()[0].notes, "");
    }

    #[test]
    fn a_check_can_be_deleted_and_all_of_them_cleared() {
        let dir = temp_dir();
        let store = FileStore::new(dir.to_path_buf());
        store.add_check(&check_at("2026-10-08T10:00:00Z"));
        store.add_check(&check_at("2026-10-09T10:00:00Z"));
        store.delete_check("2026-10-08T10:00:00Z");
        assert_eq!(ids(&store), ["2026-10-09T10:00:00Z"]);
        store.delete_check("not there");
        assert_eq!(ids(&store).len(), 1);
        store.clear_checks();
        assert!(store.load_checks().is_empty());
        assert!(!dir.join("checks.json").exists());
    }

    #[test]
    fn client_and_notes_are_saved_with_their_own_check() {
        let dir = temp_dir();
        let store = FileStore::new(dir.to_path_buf());
        store.add_check(&check_at("2026-10-08T10:00:00Z"));
        store.add_check(&check_at("2026-10-09T10:00:00Z"));
        store.set_check_notes("2026-10-08T10:00:00Z", "Mario Rossi", "New SSD fitted");
        let checks = store.load_checks();
        assert_eq!((checks[1].client.as_str(), checks[1].notes.as_str()), ("Mario Rossi", "New SSD fitted"));
        assert_eq!((checks[0].client.as_str(), checks[0].notes.as_str()), ("", ""));
    }

    #[test]
    fn two_windows_never_lose_each_others_checks() {
        let dir = temp_dir();
        let (a, b) = (FileStore::new(dir.to_path_buf()), FileStore::new(dir.to_path_buf()));
        a.add_check(&check_at("2026-10-08T10:00:00Z"));
        b.add_check(&check_at("2026-10-09T10:00:00Z"));
        assert_eq!(ids(&a), ["2026-10-09T10:00:00Z", "2026-10-08T10:00:00Z"]);
        b.delete_check("2026-10-08T10:00:00Z");
        a.set_check_notes("2026-10-09T10:00:00Z", "Bianchi", "");
        assert_eq!(b.load_checks()[0].client, "Bianchi");
        assert!(!dir.join("checks.lock").exists(), "the lock goes when it's let go");
    }

    #[test]
    fn the_logo_is_kept_beside_the_settings_and_can_be_removed() {
        let dir = temp_dir();
        let store = FileStore::new(dir.to_path_buf());
        assert_eq!(store.load_logo(), None);
        store.save_logo(b"\x89PNG not really");
        assert_eq!(store.load_logo().as_deref(), Some(&b"\x89PNG not really"[..]));
        assert!(dir.join("logo.png").exists());
        store.clear_logo();
        assert_eq!(store.load_logo(), None);
    }

    #[test]
    fn technician_details_round_trip_and_older_settings_still_load() {
        let dir = temp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        // A settings file from 1.0, before there were any.
        std::fs::write(dir.join("settings.json"), r#"{"remember":true,"theme":"dark"}"#).unwrap();
        let store = FileStore::new(dir.to_path_buf());
        let s = store.load_settings();
        assert_eq!((s.theme, s.technician.clone()), (ThemeChoice::Dark, crate::state::Technician::default()));
        let mine = crate::state::Technician { business: "PC Rossi".into(), name: "Mario".into(), contact: "055 123456".into() };
        store.save_settings(&Settings { technician: mine.clone(), ..s });
        assert_eq!(store.load_settings().technician, mine);
        // Half a block, or the wrong type, keeps what it can.
        std::fs::write(dir.join("settings.json"), r#"{"technician":{"business":"PC Rossi","name":3}}"#).unwrap();
        assert_eq!(store.load_settings().technician, crate::state::Technician::default());
        std::fs::write(dir.join("settings.json"), r#"{"technician":{"business":"PC Rossi"}}"#).unwrap();
        assert_eq!(store.load_settings().technician.business, "PC Rossi");
    }

    #[test]
    fn links_round_trip_and_expired_ones_are_forgotten() {
        let dir = temp_dir();
        let store = FileStore::new(dir.to_path_buf());
        let link = |id: &str, expires: &str| SavedLink { id: id.into(), url: format!("https://fettle.page/r/{id}"), delete_token: "t".into(), expires_at: expires.into() };
        store.save_links(&[link("a", "2026-10-01T00:00:00Z"), link("b", "2026-11-01T00:00:00Z")]);
        let now = fettle_verdicts::fmt::parse_rfc3339("2026-10-09T00:00:00Z").unwrap();
        assert_eq!(store.load_links(now), vec![link("b", "2026-11-01T00:00:00Z")]);
        // The expired one is gone from the file too.
        assert_eq!(FileStore::new(dir.to_path_buf()).load_links(0).len(), 1);
    }

    #[test]
    fn two_windows_never_lose_each_others_links() {
        // Both windows loaded the same (empty) list at launch; each shares a link, then one deletes.
        let dir = temp_dir();
        let (a, b) = (FileStore::new(dir.to_path_buf()), FileStore::new(dir.to_path_buf()));
        let link = |id: &str| SavedLink { id: id.into(), url: format!("https://fettle.page/r/{id}"), delete_token: format!("token-{id}"), expires_at: "2026-11-01T00:00:00Z".into() };
        let ids = |s: &FileStore| s.load_links(0).into_iter().map(|l| l.id).collect::<Vec<_>>();
        a.add_link(&link("a"), 0);
        b.add_link(&link("b"), 0);
        assert_eq!(ids(&a), ["a", "b"], "b's link didn't replace a's");
        assert_eq!(a.load_links(0)[0].delete_token, "token-a");
        b.add_link(&link("b"), 0);
        assert_eq!(ids(&a), ["a", "b"], "never twice");
        b.forget_links(&["a".to_string()], 0);
        assert_eq!(ids(&a), ["b"]);
        a.forget_links(&["gone".to_string()], 0);
        assert_eq!(ids(&b), ["b"]);
    }

    #[test]
    fn two_windows_saving_at_the_same_moment_keep_both_links() {
        // Window A reads the list and takes a moment to save; window B shares a link meanwhile.
        let dir = temp_dir();
        let link = |id: &str| SavedLink { id: id.into(), url: format!("https://fettle.page/r/{id}"), delete_token: format!("token-{id}"), expires_at: "2026-11-01T00:00:00Z".into() };
        let a = FileStore::new(dir.to_path_buf());
        let slow = std::thread::spawn(move || {
            a.with_links_locked(&mut || {
                let mut links = a.load_links(0);
                std::thread::sleep(std::time::Duration::from_millis(300));
                links.push(link("a"));
                a.save_links(&links);
            })
        });
        std::thread::sleep(std::time::Duration::from_millis(50));
        FileStore::new(dir.to_path_buf()).add_link(&link("b"), 0);
        slow.join().unwrap();
        let ids: Vec<String> = FileStore::new(dir.to_path_buf()).load_links(0).into_iter().map(|l| l.id).collect();
        assert_eq!(ids, ["a", "b"]);
        assert!(!dir.join("links.lock").exists(), "the lock goes when it's let go");
    }

    #[test]
    fn another_windows_half_written_file_never_gets_in_the_way() {
        // Another Fettle window is writing links.json at the same moment: its temporary file is its own.
        let dir = temp_dir();
        std::fs::create_dir_all(dir.join("links.json.tmp")).unwrap();
        let store = FileStore::new(dir.to_path_buf());
        let link = SavedLink { id: "a".into(), url: "https://fettle.page/r/a".into(), delete_token: "token-a".into(), expires_at: "2026-11-01T00:00:00Z".into() };
        store.add_link(&link, 0);
        assert_eq!(store.load_links(0), vec![link]);
        let left: Vec<String> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.ends_with(".tmp") && n != "links.json.tmp").collect();
        assert!(left.is_empty(), "{left:?}");
    }

    #[test]
    fn broken_links_are_ignored() {
        let dir = temp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("links.json"), r#"[{"id": 1}, "x"]"#).unwrap();
        assert_eq!(FileStore::new(dir.to_path_buf()).load_links(0), vec![]);
    }
}
