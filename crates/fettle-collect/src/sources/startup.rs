use super::windows::reg_value_to_json;
use serde_json::{Value, json};
use std::path::PathBuf;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
use winreg::{HKEY, RegKey};

const RUN_KEYS: &[(HKEY, &str, &str)] = &[
    (HKEY_CURRENT_USER, "HKCU", r"Software\Microsoft\Windows\CurrentVersion\Run"),
    (HKEY_CURRENT_USER, "HKCU", r"Software\Microsoft\Windows\CurrentVersion\RunOnce"),
    (HKEY_LOCAL_MACHINE, "HKLM", r"Software\Microsoft\Windows\CurrentVersion\Run"),
    (HKEY_LOCAL_MACHINE, "HKLM", r"Software\Microsoft\Windows\CurrentVersion\RunOnce"),
    (HKEY_LOCAL_MACHINE, "HKLM", r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Run"),
    (HKEY_LOCAL_MACHINE, "HKLM", r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\RunOnce"),
];
const APPROVED_BASE: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved";
const APPROVED_SUBKEYS: &[&str] = &["Run", "Run32", "StartupFolder"];

/// Task Manager's enabled/disabled flag: the low bit of the first byte set means disabled.
pub fn startup_approved_state(bytes: &[u8]) -> Option<bool> {
    bytes.first().map(|b| b & 1 == 0)
}

/// A missing key or folder just means "nothing there". Anything else (access denied)
/// is kept, so it can't masquerade as an empty list.
pub fn open_error(e: &std::io::Error) -> Option<String> {
    (e.kind() != std::io::ErrorKind::NotFound).then(|| e.to_string())
}

pub fn startup_registry() -> Result<Value, String> {
    let mut out = Vec::new();
    for &(hive, hive_name, path) in RUN_KEYS {
        let key = match RegKey::predef(hive).open_subkey(path) {
            Ok(key) => key,
            Err(e) => {
                if let Some(error) = open_error(&e) {
                    out.push(json!({"hive": hive_name, "key": path, "error": error}));
                }
                continue;
            }
        };
        for (name, value) in key.enum_values().flatten() {
            out.push(json!({"hive": hive_name, "key": path, "name": name, "command": reg_value_to_json(&value)}));
        }
    }
    Ok(Value::Array(out))
}

pub fn startup_folders() -> Result<Value, String> {
    let folders = [
        ("user", std::env::var("APPDATA").map(|p| PathBuf::from(p).join(r"Microsoft\Windows\Start Menu\Programs\Startup"))),
        ("common", std::env::var("ProgramData").map(|p| PathBuf::from(p).join(r"Microsoft\Windows\Start Menu\Programs\StartUp"))),
    ];
    let mut out = Vec::new();
    for (label, dir) in folders {
        let Ok(dir) = dir else { continue };
        match startup_files(&dir) {
            Ok(files) => out.extend(files.into_iter().map(|file| json!({"folder": label, "file": file}))),
            Err(e) => out.extend(open_error(&e).map(|error| json!({"folder": label, "error": error}))),
        }
    }
    Ok(Value::Array(out))
}

/// The files Windows starts from one Startup folder.
fn startup_files(dir: &std::path::Path) -> std::io::Result<Vec<String>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)?.flatten() {
        if entry.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let file = entry.file_name().to_string_lossy().into_owned();
        if !file.eq_ignore_ascii_case("desktop.ini") {
            files.push(file);
        }
    }
    Ok(files)
}

pub fn startup_approved() -> Result<Value, String> {
    let mut out = Vec::new();
    for (hive, hive_name) in [(HKEY_CURRENT_USER, "HKCU"), (HKEY_LOCAL_MACHINE, "HKLM")] {
        for sub in APPROVED_SUBKEYS {
            let path = format!(r"{APPROVED_BASE}\{sub}");
            let key = match RegKey::predef(hive).open_subkey(&path) {
                Ok(key) => key,
                Err(e) => {
                    if let Some(error) = open_error(&e) {
                        out.push(json!({"hive": hive_name, "key": sub, "error": error}));
                    }
                    continue;
                }
            };
            for (name, value) in key.enum_values().flatten() {
                let raw_hex: String = value.bytes.iter().map(|b| format!("{b:02x}")).collect();
                out.push(json!({
                    "hive": hive_name,
                    "key": sub,
                    "name": name,
                    "enabled": startup_approved_state(&value.bytes),
                    "raw_hex": raw_hex,
                }));
            }
        }
    }
    Ok(Value::Array(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch folder under target/tmp (never Windows' Temp), gone when the test is, passed or failed.
    struct Scratch(std::path::PathBuf);

    impl Scratch {
        fn new(name: &str) -> Scratch {
            let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/fettle-collect").join(name);
            let _ = std::fs::remove_dir_all(&dir);
            Scratch(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_folder_inside_the_startup_folder_is_not_an_app() {
        // Windows starts the files in the Startup folder, not what's inside a folder there.
        let dir = Scratch::new("startup-folder");
        std::fs::create_dir_all(dir.0.join("Old shortcuts")).unwrap();
        std::fs::write(dir.0.join("Notes.lnk"), b"").unwrap();
        std::fs::write(dir.0.join("desktop.ini"), b"").unwrap();
        assert_eq!(startup_files(&dir.0).unwrap(), ["Notes.lnk"]);
    }

    #[test]
    fn not_found_is_silent_other_errors_are_kept() {
        use std::io::{Error, ErrorKind};
        assert_eq!(open_error(&Error::from(ErrorKind::NotFound)), None);
        assert!(open_error(&Error::from_raw_os_error(5)).is_some());
    }

    #[test]
    fn startup_approved_first_byte_decides() {
        assert_eq!(startup_approved_state(&[0x02, 0, 0]), Some(true));
        assert_eq!(startup_approved_state(&[0x06, 0, 0]), Some(true));
        assert_eq!(startup_approved_state(&[0x03, 0, 0]), Some(false));
        assert_eq!(startup_approved_state(&[0x07, 0, 0]), Some(false));
        assert_eq!(startup_approved_state(&[]), None);
    }
}
