//! The Deep check. This window stays open and starts an elevated helper (`fettle.exe --deep-scan
//! <file>`, no window) that scans as administrator, where the scan reads more (disk health, the TPM
//! chip), and writes its report to a file. The app reads it, keeps the user's own startup apps, and
//! shows it here.
//!
//! A standard user approves the prompt with an administrator's password, so the helper runs as that
//! administrator's account: anything per-user it read (startup apps) is someone else's, and anything
//! it wrote to "its" AppData would land in their profile. So the helper only scans and writes one
//! report file, and the user's settings, remembered check and shared links never leave their own
//! profile.

use crate::state::DeepError;
use fettle_facts::Report;
use std::io::Write;
use std::path::{Path, PathBuf};

/// The argument the elevated helper starts with, followed by the report file it writes.
pub const SCAN_ARG: &str = "--deep-scan";

/// What `--deep-scan` asks of this copy of fettle.exe.
#[derive(Debug, PartialEq, Eq)]
pub enum Helper {
    /// Not asked: this is the app.
    NotAsked,
    /// Run as the Deep check's helper and write this report file.
    Write(PathBuf),
    /// Asked with a file it won't write: stop there, and never open the app (it would run as
    /// administrator).
    Refuse,
}

/// What the command line asks of this copy. The helper runs as administrator, so it only writes a new
/// `fettle-deep-*.json` at an absolute local path (never a network share or a `\\?\` or `\\.\` device path).
pub fn helper_target(args: impl IntoIterator<Item = String>) -> Helper {
    let mut args = args.into_iter().skip(1);
    while let Some(a) = args.next() {
        if a == SCAN_ARG {
            return match args.next().map(PathBuf::from) {
                Some(path) if acceptable(&path) => Helper::Write(path),
                _ => Helper::Refuse,
            };
        }
    }
    Helper::NotAsked
}

fn acceptable(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    !path.as_os_str().to_string_lossy().starts_with(r"\\") && path.is_absolute() && name.starts_with("fettle-deep-") && name.ends_with(".json")
}

/// The report file the helper is asked to write into `dir`, checked the way the helper will check it,
/// so a folder it would refuse fails here, before Windows asks for permission.
pub fn deep_report_path(dir: &Path, pid: u32, nonce: u128) -> Result<PathBuf, DeepError> {
    let path = dir.join(format!("fettle-deep-{pid}-{nonce}.json"));
    if acceptable(&path) && !through_a_link(dir).unwrap_or(true) { Ok(path) } else { Err(DeepError::Failed) }
}

/// Writes `report` to a file that must not exist yet (never overwrites one).
pub fn write_new(path: &Path, report: &Report) -> std::io::Result<()> {
    let json = serde_json::to_vec(report).map_err(std::io::Error::other)?;
    std::fs::OpenOptions::new().write(true).create_new(true).open(path)?.write_all(&json)
}

/// Writes the helper's report: only a new file, and only into real folders all the way up (a junction
/// or link anywhere on the path could point anywhere the administrator can write).
pub fn write_report(path: &Path, report: &Report) -> std::io::Result<()> {
    let folder = path.parent().ok_or_else(|| std::io::Error::other("no folder"))?;
    if through_a_link(folder)? {
        return Err(std::io::Error::other("the path goes through a junction or a link"));
    }
    write_new(path, report)
}

/// Whether `folder` or any folder above it is a junction or a symbolic link. Other reparse points,
/// such as OneDrive's files-on-demand folders, are real folders and don't count.
fn through_a_link(folder: &Path) -> std::io::Result<bool> {
    for f in folder.ancestors().filter(|f| !f.as_os_str().is_empty()) {
        if std::fs::symlink_metadata(f)?.file_type().is_symlink() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// What the helper hands the app: its scan without startup apps, which would be the administrator's
/// (the app uses the user's own).
pub fn for_the_app(mut report: Report) -> Report {
    report.facts.startup = None;
    report
}

/// How long the app waits for the helper, from when Windows starts it.
pub const APP_WAIT: std::time::Duration = std::time::Duration::from_secs(120);
/// How long the helper may take and still write its report: well inside the app's wait, so a report
/// it writes is one the app reads (and deletes), never one left behind after the app gave up.
pub const HELPER_DEADLINE: std::time::Duration = std::time::Duration::from_secs(110);

/// The helper's whole job, run before any window exists: scan, write the report, and say how it
/// went in the exit code (0 written, 1 not).
pub fn run_helper(path: &Path) -> i32 {
    let started = std::time::Instant::now();
    let report = for_the_app(crate::scan::scan_quietly());
    finish_helper(path, &report, started.elapsed())
}

/// Writes the helper's report if it's in time (`took` is how long the scan ran), and says how it went.
pub fn finish_helper(path: &Path, report: &Report, took: std::time::Duration) -> i32 {
    if took >= HELPER_DEADLINE {
        return 1;
    }
    if write_report(path, report).is_ok() { 0 } else { 1 }
}

/// Clears Deep-check report files a helper wrote after the app had stopped waiting (it gave up, or
/// was closed). Only ones older than ten minutes: a newer one may be another window's check.
pub fn remove_leftovers(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(600);
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let old = entry.metadata().and_then(|m| m.modified()).is_ok_and(|t| t < cutoff);
        if name.starts_with("fettle-deep-") && name.ends_with(".json") && old {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// The Deep check's report: the elevated scan's, except the user's own startup apps, which come
/// from their own scan (the elevated one may have read another account's).
pub fn merge_deep(user: Report, elevated: Report) -> Report {
    let mut deep = elevated;
    deep.facts.startup = user.facts.startup;
    deep
}

/// ERROR_CANCELLED (1223) as an HRESULT: the user declined the prompt.
const CANCELLED_HRESULT: i32 = 0x8007_04C7_u32 as i32;

pub fn error_from(hresult: i32) -> DeepError {
    if hresult == CANCELLED_HRESULT { DeepError::Cancelled } else { DeepError::Failed }
}

/// The helper's run, from the user's side: Windows asks first (the prompt belongs to `owner`, the
/// app's window, 0 for none), then the helper scans and writes its report into Fettle's own folder,
/// which this reads and deletes. Blocks until the helper is done; run it off the window's thread.
#[cfg(windows)]
pub fn deep_scan(owner: isize) -> Result<(Report, f64), DeepError> {
    let dir = crate::store::FileStore::default_dir();
    // A relative folder is refused below anyway; don't create one in whatever folder this started in.
    if !dir.is_absolute() {
        return Err(DeepError::Failed);
    }
    std::fs::create_dir_all(&dir).map_err(|_| DeepError::Failed)?;
    let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let path = deep_report_path(&dir, std::process::id(), nonce)?;
    let result = run_elevated(owner, &path).and_then(|secs| {
        let text = std::fs::read(&path).map_err(|_| DeepError::Failed)?;
        Ok((serde_json::from_slice::<Report>(&text).map_err(|_| DeepError::Failed)?, secs))
    });
    let _ = std::fs::remove_file(&path);
    result
}

/// Starts the helper as administrator and waits (up to two minutes) for it to finish: how long it
/// ran, from when Windows started it (so not the time spent on the prompt).
#[cfg(windows)]
fn run_elevated(owner: isize, path: &Path) -> Result<f64, DeepError> {
    use windows::Win32::Foundation::{CloseHandle, HWND, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
    use windows::Win32::UI::Shell::{SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW};
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
    use windows::core::{HSTRING, PCWSTR, w};

    let exe = std::env::current_exe().map_err(|_| DeepError::Failed)?;
    let file = HSTRING::from(exe.as_os_str());
    let params = HSTRING::from(format!("{SCAN_ARG} \"{}\"", path.display()));
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC | SEE_MASK_NOCLOSEPROCESS,
        hwnd: HWND(owner as _),
        lpVerb: w!("runas"),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    // SAFETY: `info` is fully initialised and the strings it points to outlive the call; the process
    // handle it returns is waited on and closed here.
    unsafe {
        ShellExecuteExW(&mut info).map_err(|e| error_from(e.code().0))?;
        if info.hProcess.is_invalid() {
            return Err(DeepError::Failed);
        }
        let started = std::time::Instant::now();
        let finished = WaitForSingleObject(info.hProcess, APP_WAIT.as_millis() as u32) == WAIT_OBJECT_0;
        let mut code = 1u32;
        let _ = GetExitCodeProcess(info.hProcess, &mut code);
        let _ = CloseHandle(info.hProcess);
        if finished && code == 0 { Ok(started.elapsed().as_secs_f64()) } else { Err(DeepError::Failed) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fettle_facts::{Section, Startup, StartupItem};

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_helper_only_writes_a_new_deep_report_file() {
        let dir = std::env::temp_dir();
        let good = dir.join("fettle-deep-123-456.json");
        assert_eq!(helper_target(args(&["fettle.exe", "--deep-scan", good.to_str().unwrap()])), Helper::Write(good.clone()));
        // Any other file name is refused: the helper runs as administrator.
        for bad in [r"C:\Windows\System32\drivers\etc\hosts", r"C:\Users\x\fettle-deep-1.txt", r"C:\Users\x\settings.json", "fettle-deep-1-1.json", r"Fettle data\fettle-deep-1-1.json"] {
            assert_eq!(helper_target(args(&["fettle.exe", "--deep-scan", bad])), Helper::Refuse, "{bad}");
        }
        assert_eq!(helper_target(args(&["fettle.exe", "--deep-scan"])), Helper::Refuse);
        assert_eq!(helper_target(args(&["fettle.exe"])), Helper::NotAsked);
        // The program's own path never counts.
        assert_eq!(helper_target(args(&["--deep-scan", good.to_str().unwrap()])), Helper::NotAsked);
    }

    #[test]
    fn the_helper_never_overwrites_a_file() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/fettle-elevate");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fettle-deep-1-1.json");
        let _ = std::fs::remove_file(&path);
        let report = Report { schema_version: 1, app_version: "1.0.0".into(), engine_version: None, collected_at: "2026-10-10T10:00:00Z".into(), elevated: true, facts: Default::default() };
        assert!(write_new(&path, &report).is_ok());
        assert!(write_new(&path, &report).is_err(), "already there");
        let _ = std::fs::remove_file(&path);
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/fettle-elevate").join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn a_report() -> Report {
        Report { schema_version: 1, app_version: "1.0.0".into(), engine_version: None, collected_at: "2026-10-10T10:00:00Z".into(), elevated: true, facts: Default::default() }
    }

    #[test]
    fn the_helper_never_writes_to_a_network_path() {
        for bad in [r"\\host\share\fettle-deep-1.json", r"\\?\UNC\host\share\fettle-deep-1.json", r"\\?\C:\Users\x\fettle-deep-1.json", r"\\.\pipe\fettle-deep-1.json"] {
            assert_eq!(helper_target(args(&["fettle.exe", "--deep-scan", bad])), Helper::Refuse, "{bad}");
        }
    }

    #[test]
    fn a_folder_the_helper_would_refuse_is_caught_before_windows_asks() {
        // Portable data on a network share, or a relative folder when LOCALAPPDATA isn't set: the app
        // says the Deep check failed without a prompt, rather than starting a helper that can't write.
        assert_eq!(deep_report_path(Path::new(r"\\nas\tools\Fettle data"), 7, 9), Err(DeepError::Failed));
        assert_eq!(deep_report_path(Path::new("Fettle data"), 7, 9), Err(DeepError::Failed));
        let dir = scratch("target");
        let path = deep_report_path(&dir, 7, 9).expect("a local folder");
        assert_eq!(path, dir.join("fettle-deep-7-9.json"));
        assert_eq!(helper_target(args(&["fettle.exe", "--deep-scan", path.to_str().unwrap()])), Helper::Write(path.clone()));
    }

    #[test]
    fn the_helper_never_writes_through_a_junction() {
        let dir = scratch("junction");
        let real = dir.join("real");
        std::fs::create_dir_all(&real).unwrap();
        let link = dir.join("link");
        // mklink reads a "/" in an unquoted path as a switch (a path without spaces isn't quoted), so
        // hand it backslashes only.
        let windows_path = |p: &std::path::Path| p.to_string_lossy().replace('/', "\\");
        let made = std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(windows_path(&link)).arg(windows_path(&real)).output().unwrap();
        assert!(made.status.success(), "{made:?}");
        assert!(write_report(&link.join("fettle-deep-1-1.json"), &a_report()).is_err(), "a junction may point anywhere");
        assert!(!real.join("fettle-deep-1-1.json").exists());
        assert!(write_report(&real.join("fettle-deep-1-1.json"), &a_report()).is_ok());
        // Further up counts too: a junction above the report's own folder.
        std::fs::create_dir_all(real.join("Fettle data")).unwrap();
        assert!(write_report(&link.join("Fettle data").join("fettle-deep-1-2.json"), &a_report()).is_err(), "a junction further up");
        assert!(!real.join("Fettle data").join("fettle-deep-1-2.json").exists());
        assert_eq!(deep_report_path(&link.join("Fettle data"), 1, 2), Err(DeepError::Failed), "caught before Windows asks");
        // The junction goes with the test: tools that clean folders (git's worktree removal) stop at one.
        std::fs::remove_dir(&link).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_helper_never_writes_startup_apps() {
        // They'd be the administrator's (the app uses the user's own anyway).
        let mut report = a_report();
        report.facts.startup = startup(&["AdminTool"]);
        let dir = scratch("startup");
        let path = dir.join("fettle-deep-1-2.json");
        write_report(&path, &for_the_app(report)).unwrap();
        let written: Report = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(written.facts.startup, None);
    }

    #[test]
    fn a_helper_that_finishes_after_the_app_stopped_waiting_writes_nothing() {
        // It stops in time for the app to read the file and delete it, rather than leave one behind.
        assert!(HELPER_DEADLINE + std::time::Duration::from_secs(5) < APP_WAIT, "a margin for the helper starting late");
        let dir = scratch("late");
        let in_time = dir.join("fettle-deep-1-1.json");
        assert_eq!(finish_helper(&in_time, &a_report(), std::time::Duration::from_secs(30)), 0);
        assert!(in_time.exists());
        let late = dir.join("fettle-deep-1-2.json");
        assert_eq!(finish_helper(&late, &a_report(), HELPER_DEADLINE), 1);
        assert!(!late.exists(), "the app has given up on it");
    }

    #[test]
    fn old_deep_files_are_cleared_at_launch() {
        let dir = scratch("leftovers");
        let old = dir.join("fettle-deep-1-1.json");
        let fresh = dir.join("fettle-deep-2-2.json");
        let other = dir.join("settings.json");
        for p in [&old, &fresh, &other] {
            std::fs::write(p, "{}").unwrap();
        }
        let hour_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        std::fs::File::options().write(true).open(&old).unwrap().set_modified(hour_ago).unwrap();
        remove_leftovers(&dir);
        assert!(!old.exists(), "a Deep check that never came back");
        assert!(fresh.exists(), "may belong to a Deep check running in another window");
        assert!(other.exists());
    }

    fn startup(names: &[&str]) -> Option<Section<Startup>> {
        let items = names.iter().map(|n| StartupItem { name: Some(n.to_string()), ..Default::default() }).collect();
        Some(Section { check: fettle_facts::Check { status: fettle_facts::CheckStatus::Ok, detail: None }, data: Some(Startup { items }) })
    }

    #[test]
    fn an_elevated_result_never_replaces_the_users_startup_apps() {
        // A standard user's Deep check runs elevated as an administrator's account: its startup list
        // is the administrator's. The receipt keeps the user's own.
        let mut user = Report { schema_version: 1, app_version: "1.0.0".into(), engine_version: None, collected_at: "2026-10-10T10:00:00Z".into(), elevated: false, facts: Default::default() };
        user.facts.startup = startup(&["OneDrive", "Spotify"]);
        let mut admin = Report { elevated: true, collected_at: "2026-10-10T10:00:05Z".into(), ..user.clone() };
        admin.facts.startup = startup(&["AdminTool", "Backup Agent", "VPN"]);
        admin.facts.windows = None;
        let deep = merge_deep(user.clone(), admin.clone());
        assert_eq!(deep.facts.startup, user.facts.startup);
        assert!(deep.elevated, "everything else is the Deep check's");
        assert_eq!(deep.collected_at, admin.collected_at);
        // Even when the user's own scan couldn't read startup, the administrator's isn't used.
        let mut blind = user.clone();
        blind.facts.startup = None;
        assert_eq!(merge_deep(blind, admin).facts.startup, None);
    }
}
