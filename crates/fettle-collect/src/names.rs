//! The names that must not leave this PC: every account's name and the computer name.
//! They feed `fettle_facts::redact::Redactor`.

/// Profile folders that belong to no one in particular.
const SHARED_PROFILES: &[&str] = &["Public", "Default", "Default User", "All Users"];

/// The account folder name of a personal profile path, or None for system and shared profiles.
pub fn profile_leaf(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches('\\');
    let (parent, leaf) = trimmed.rsplit_once('\\')?;
    let is_users_dir = parent.rsplit('\\').next().is_some_and(|p| p.eq_ignore_ascii_case("Users"));
    let shared = SHARED_PROFILES.iter().any(|p| p.eq_ignore_ascii_case(leaf));
    (is_users_dir && !leaf.is_empty() && !shared).then(|| leaf.to_string())
}

/// The privacy pass for this PC, with every name it goes by.
#[cfg(windows)]
pub fn redactor() -> fettle_facts::redact::Redactor {
    let (users, hosts) = machine_names();
    let mut hosts = hosts.into_iter();
    let first = hosts.next().unwrap_or_default();
    hosts.fold(fettle_facts::redact::Redactor::new(users, first), |r, h| r.with_hostname(h))
}

/// Every account name on this PC, not just the one running the scan: an admin run under IT
/// credentials still sees the employee's startup entries. Plus the computer's names: the short
/// one in `COMPUTERNAME` (cut to 15 characters) and the full DNS one.
#[cfg(windows)]
pub fn machine_names() -> (Vec<String>, Vec<String>) {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;
    const PROFILE_LIST: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList";

    let mut names: Vec<String> = Vec::new();
    names.extend(std::env::var("USERNAME").ok());
    names.extend(std::env::var("USERPROFILE").ok().and_then(|p| profile_leaf(&p)));
    if let Ok(profiles) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(PROFILE_LIST) {
        for sid in profiles.enum_keys().flatten() {
            let path: Option<String> = profiles.open_subkey(&sid).ok().and_then(|k| k.get_value("ProfileImagePath").ok());
            names.extend(path.and_then(|p| profile_leaf(&p)));
        }
    }
    // Redactor drops duplicates, empty names and names too short to match safely.
    let hosts = [std::env::var("COMPUTERNAME").ok(), dns_hostname()].into_iter().flatten().collect();
    (names, hosts)
}

#[cfg(windows)]
fn dns_hostname() -> Option<String> {
    use windows::Win32::System::SystemInformation::{ComputerNameDnsHostname, GetComputerNameExW};
    use windows::core::PWSTR;
    let mut len = 0u32;
    // The first call fails on purpose and reports the size needed.
    let _ = unsafe { GetComputerNameExW(ComputerNameDnsHostname, None, &mut len) };
    let mut buf = vec![0u16; len as usize];
    unsafe { GetComputerNameExW(ComputerNameDnsHostname, Some(PWSTR(buf.as_mut_ptr())), &mut len) }.ok()?;
    Some(String::from_utf16_lossy(&buf[..len as usize])).filter(|h| !h.is_empty())
}

#[cfg(test)]
mod tests {
    use super::profile_leaf;

    #[test]
    fn profile_leaf_keeps_only_personal_profiles() {
        assert_eq!(profile_leaf(r"C:\Users\jdoe"), Some("jdoe".into()));
        assert_eq!(profile_leaf(r"C:\Users\jdoe.CONTOSO\"), Some("jdoe.CONTOSO".into()));
        assert_eq!(profile_leaf(r"D:\users\Ana Lima"), Some("Ana Lima".into()));
        assert_eq!(profile_leaf(r"C:\WINDOWS\system32\config\systemprofile"), None);
        assert_eq!(profile_leaf(r"C:\Users\Public"), None);
        assert_eq!(profile_leaf(r"C:\Users\"), None);
        assert_eq!(profile_leaf(""), None);
    }

    #[cfg(windows)]
    #[test]
    fn this_pcs_names_are_found_and_then_hidden() {
        let (users, hosts) = super::machine_names();
        let me = std::env::var("USERNAME").unwrap();
        let host = std::env::var("COMPUTERNAME").unwrap();
        assert!(users.iter().any(|u| u.eq_ignore_ascii_case(&me)));
        assert!(hosts.iter().any(|h| h.eq_ignore_ascii_case(&host)));
        // The full DNS name too, which `hostname` prints (COMPUTERNAME stops at 15 characters).
        let dns = String::from_utf8(std::process::Command::new("hostname").output().unwrap().stdout).unwrap();
        assert!(hosts.iter().any(|h| h.eq_ignore_ascii_case(dns.trim())), "{hosts:?} vs {dns}");
        let r = super::redactor();
        let text = format!(r"{host}\{me} C:\Users\{me}\AppData");
        assert_eq!(r.text(&text), r"<pc-name>\<user> C:\Users\<user>\AppData");
    }
}
