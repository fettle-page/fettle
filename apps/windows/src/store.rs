//! What Fettle keeps on this PC, in `%LOCALAPPDATA%\Fettle\`: the settings, the last check (the
//! same redacted report the receipt shows) and the share links it can still delete. A file that is
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

pub trait Store {
    fn load_settings(&self) -> Settings;
    fn save_settings(&self, settings: &Settings);
    fn load_last(&self) -> Option<Report>;
    fn save_last(&self, report: &Report);
    fn clear_last(&self);
    /// The links still alive at `now_ms`; expired ones are forgotten.
    fn load_links(&self, now_ms: i64) -> Vec<SavedLink>;
    fn save_links(&self, links: &[SavedLink]);
    /// The last update check (a missing or bad file is "never asked").
    fn load_update(&self) -> crate::update::UpdateRecord;
    fn save_update(&self, record: &crate::update::UpdateRecord);
}

/// The real store: JSON files in one folder.
pub struct FileStore {
    dir: PathBuf,
}

impl FileStore {
    pub fn new(dir: PathBuf) -> FileStore {
        FileStore { dir }
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
const LAST: &str = "last-report.json";
const LINKS: &str = "links.json";
const UPDATE: &str = "update.json";

impl FileStore {
    fn read(&self, name: &str) -> Option<serde_json::Value> {
        serde_json::from_str(&std::fs::read_to_string(self.dir.join(name)).ok()?).ok()
    }

    /// Best effort: a full disk or a locked file just means it isn't kept this time.
    fn write(&self, name: &str, value: &impl Serialize) {
        let Ok(text) = serde_json::to_string(value) else { return };
        let _ = std::fs::create_dir_all(&self.dir);
        // Written aside, then moved over the old file, so a crash never leaves half a file.
        let tmp = self.dir.join(format!("{name}.tmp"));
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, self.dir.join(name));
        }
    }

    fn remove(&self, name: &str) {
        let _ = std::fs::remove_file(self.dir.join(name));
    }
}

/// One field of a JSON object, or the default when it's missing or of the wrong type.
fn field<T: serde::de::DeserializeOwned>(object: &serde_json::Value, name: &str, default: T) -> T {
    object.get(name).and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or(default)
}

impl Store for FileStore {
    fn load_settings(&self) -> Settings {
        let d = Settings::default();
        let Some(o) = self.read(SETTINGS).filter(serde_json::Value::is_object) else { return d };
        Settings {
            remember: field(&o, "remember", d.remember),
            updates: field(&o, "updates", d.updates),
            update_every: field(&o, "update_every", d.update_every),
            show_private: field(&o, "show_private", d.show_private),
            lang: field(&o, "lang", d.lang),
            theme: field(&o, "theme", d.theme),
        }
    }

    fn save_settings(&self, settings: &Settings) {
        self.write(SETTINGS, settings);
    }

    fn load_last(&self) -> Option<Report> {
        if !self.dir.join(LAST).exists() {
            return None;
        }
        let usable = self
            .read(LAST)
            .filter(|v| v["schema_version"].as_u64().is_some_and(|n| n <= u64::from(SCHEMA_VERSION)))
            .and_then(|v| serde_json::from_value::<Report>(v).ok())
            // A report this version can't turn into a receipt is forgotten too.
            .filter(|r| fettle_receipt::receipt_for(r, fettle_verdicts::Locale::En, fettle_receipt::Zone::Utc).is_ok());
        if usable.is_none() {
            self.clear_last();
        }
        usable
    }

    fn save_last(&self, report: &Report) {
        self.write(LAST, report);
    }

    fn clear_last(&self) {
        self.remove(LAST);
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
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::state::ThemeChoice;
    use fettle_verdicts::Locale;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A fresh, empty folder for one test.
    /// A fresh scratch folder under target/ (never Windows' Temp); each run clears the last run's.
    pub(crate) fn temp_dir() -> PathBuf {
        static ROOT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
        static N: AtomicUsize = AtomicUsize::new(0);
        let root = ROOT.get_or_init(|| {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/fettle-unit");
            let _ = std::fs::remove_dir_all(&root);
            root
        });
        root.join(N.fetch_add(1, Ordering::Relaxed).to_string())
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
    fn settings_round_trip_and_start_from_defaults() {
        let store = FileStore::new(temp_dir());
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
        let s = FileStore::new(dir.clone()).load_settings();
        assert_eq!((s.remember, s.theme, s.lang), (true, ThemeChoice::Dark, None));
        std::fs::write(dir.join("settings.json"), "{not json").unwrap();
        assert_eq!(FileStore::new(dir).load_settings(), Settings::default());
    }

    #[test]
    fn the_last_check_round_trips_and_clears() {
        let store = FileStore::new(temp_dir());
        assert_eq!(store.load_last(), None);
        let r = dump_report();
        store.save_last(&r);
        assert_eq!(store.load_last(), Some(r));
        store.clear_last();
        assert_eq!(store.load_last(), None);
    }

    #[test]
    fn a_broken_or_newer_last_check_is_forgotten() {
        let dir = temp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("last-report.json"), "[1, 2").unwrap();
        assert_eq!(FileStore::new(dir.clone()).load_last(), None);
        assert!(!dir.join("last-report.json").exists());
        let mut newer = serde_json::to_value(dump_report()).unwrap();
        newer["schema_version"] = (SCHEMA_VERSION + 1).into();
        std::fs::write(dir.join("last-report.json"), newer.to_string()).unwrap();
        assert_eq!(FileStore::new(dir.clone()).load_last(), None);
        // A report whose scan time can't be read can't make a receipt: forgotten too.
        let mut odd = serde_json::to_value(dump_report()).unwrap();
        odd["collected_at"] = "2026-02-32T10:00:00Z".into();
        std::fs::write(dir.join("last-report.json"), odd.to_string()).unwrap();
        assert_eq!(FileStore::new(dir).load_last(), None);
    }

    #[test]
    fn links_round_trip_and_expired_ones_are_forgotten() {
        let dir = temp_dir();
        let store = FileStore::new(dir.clone());
        let link = |id: &str, expires: &str| SavedLink { id: id.into(), url: format!("https://fettle.page/r/{id}"), delete_token: "t".into(), expires_at: expires.into() };
        store.save_links(&[link("a", "2026-10-01T00:00:00Z"), link("b", "2026-11-01T00:00:00Z")]);
        let now = fettle_verdicts::fmt::parse_rfc3339("2026-10-09T00:00:00Z").unwrap();
        assert_eq!(store.load_links(now), vec![link("b", "2026-11-01T00:00:00Z")]);
        // The expired one is gone from the file too.
        assert_eq!(FileStore::new(dir).load_links(0).len(), 1);
    }

    #[test]
    fn broken_links_are_ignored() {
        let dir = temp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("links.json"), r#"[{"id": 1}, "x"]"#).unwrap();
        assert_eq!(FileStore::new(dir).load_links(0), vec![]);
    }
}
