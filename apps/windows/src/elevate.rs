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

/// The report file this copy was started to write as the Deep check's helper. Only a new
/// `fettle-deep-*.json` at an absolute path counts: the helper runs as administrator, so it never
/// writes anywhere else.
pub fn helper_target(args: impl IntoIterator<Item = String>) -> Option<PathBuf> {
    let mut args = args.into_iter().skip(1);
    while let Some(a) = args.next() {
        if a == SCAN_ARG {
            let path = PathBuf::from(args.next()?);
            let name = path.file_name()?.to_str()?;
            return (path.is_absolute() && name.starts_with("fettle-deep-") && name.ends_with(".json")).then_some(path);
        }
    }
    None
}

/// Writes `report` to a file that must not exist yet (never overwrites one).
pub fn write_new(path: &Path, report: &Report) -> std::io::Result<()> {
    let json = serde_json::to_vec(report).map_err(std::io::Error::other)?;
    std::fs::OpenOptions::new().write(true).create_new(true).open(path)?.write_all(&json)
}

/// The helper's whole job, run before any window exists: scan, write the report, and say how it
/// went in the exit code (0 written, 1 not).
pub fn run_helper(path: &Path) -> i32 {
    let report = crate::scan::scan_quietly();
    if write_new(path, &report).is_ok() { 0 } else { 1 }
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
pub fn deep_scan(owner: isize) -> Result<Report, DeepError> {
    let dir = crate::store::FileStore::default_dir();
    std::fs::create_dir_all(&dir).map_err(|_| DeepError::Failed)?;
    let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let path = dir.join(format!("fettle-deep-{}-{nonce}.json", std::process::id()));
    let result = run_elevated(owner, &path).and_then(|()| {
        let text = std::fs::read(&path).map_err(|_| DeepError::Failed)?;
        serde_json::from_slice::<Report>(&text).map_err(|_| DeepError::Failed)
    });
    let _ = std::fs::remove_file(&path);
    result
}

/// Starts the helper as administrator and waits (up to two minutes) for it to finish.
#[cfg(windows)]
fn run_elevated(owner: isize, path: &Path) -> Result<(), DeepError> {
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
        let finished = WaitForSingleObject(info.hProcess, 120_000) == WAIT_OBJECT_0;
        let mut code = 1u32;
        let _ = GetExitCodeProcess(info.hProcess, &mut code);
        let _ = CloseHandle(info.hProcess);
        if finished && code == 0 { Ok(()) } else { Err(DeepError::Failed) }
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
        assert_eq!(helper_target(args(&["fettle.exe", "--deep-scan", good.to_str().unwrap()])), Some(good.clone()));
        // Any other file name is refused: the helper runs as administrator.
        for bad in [r"C:\Windows\System32\drivers\etc\hosts", r"C:\Users\x\fettle-deep-1.txt", r"C:\Users\x\settings.json"] {
            assert_eq!(helper_target(args(&["fettle.exe", "--deep-scan", bad])), None, "{bad}");
        }
        assert_eq!(helper_target(args(&["fettle.exe", "--deep-scan"])), None);
        assert_eq!(helper_target(args(&["fettle.exe"])), None);
        // The program's own path never counts.
        assert_eq!(helper_target(args(&["--deep-scan", good.to_str().unwrap()])), None);
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
