//! The names that must not leave this PC: every account's name, full name and email address, the
//! account's domain, and the computer name. They feed `fettle_facts::redact::Redactor`.

/// The account folder name of a personal profile path, or None for system and shared profiles.
pub fn profile_leaf(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches('\\');
    let (parent, leaf) = trimmed.rsplit_once('\\')?;
    let is_users_dir = parent.rsplit('\\').next().is_some_and(|p| p.eq_ignore_ascii_case("Users"));
    let shared = fettle_facts::redact::is_shared_profile(leaf);
    (is_users_dir && !leaf.is_empty() && !shared).then(|| leaf.to_string())
}

/// The privacy pass for this PC, with every name it goes by.
#[cfg(windows)]
pub fn redactor() -> fettle_facts::redact::Redactor {
    let (mut users, hosts) = machine_names();
    let accounts = local_accounts();
    let full = full_names(&users, &accounts);
    // Every local account's name too: one renamed after its profile folder was made, or one that
    // never signed in, has a name no folder shows.
    users.extend(accounts.into_iter().map(|(name, _)| name));
    let mut hosts = hosts.into_iter();
    let first = hosts.next().unwrap_or_default();
    let r = hosts.fold(fettle_facts::redact::Redactor::new(users, first), |r, h| r.with_hostname(h));
    let r = full.into_iter().fold(r, |r, n| r.with_full_name(n));
    let r = account_emails().into_iter().fold(r, |r, e| r.with_email(e));
    domains().into_iter().fold(r, |r, d| r.with_domain(d))
}

/// The company's names: the account's domain (CONTOSO), the PC's DNS domain (corp.contoso.com), and
/// the organisations OneDrive for work or school is signed in to ("OneDrive - Contoso Ltd").
#[cfg(windows)]
fn domains() -> Vec<String> {
    use windows::Win32::System::SystemInformation::ComputerNameDnsDomain;
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let mut domains: Vec<String> = ["USERDOMAIN", "USERDNSDOMAIN"].iter().filter_map(|v| std::env::var(v).ok()).collect();
    domains.extend(computer_name(ComputerNameDnsDomain));
    if let Ok(accounts) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\Microsoft\OneDrive\Accounts") {
        for account in accounts.enum_keys().flatten().filter(|a| a.starts_with("Business")) {
            domains.extend(accounts.open_subkey(&account).ok().and_then(|k| k.get_value::<String, _>("DisplayName").ok()));
        }
    }
    domains
}

/// The full names of the people using this PC ("Mario Rossi"): the local accounts with a profile
/// folder among `people` (a Microsoft account's full name is its owner's name; accounts that
/// software creates and nobody signs in to have product names there instead), and the signed-in
/// account's from its domain or Entra ID.
#[cfg(windows)]
fn full_names(people: &[String], accounts: &[(String, Option<String>)]) -> Vec<String> {
    use windows::Win32::Security::Authentication::Identity::NameDisplay;

    let mut names: Vec<String> = user_name_ex(NameDisplay).into_iter().collect();
    for (name, full) in accounts {
        if people.iter().any(|p| p.eq_ignore_ascii_case(name)) {
            names.extend(full.clone().filter(|n| !n.trim().is_empty()));
        }
    }
    names
}

/// Every local account on this PC, with its full name when it has one.
#[cfg(windows)]
fn local_accounts() -> Vec<(String, Option<String>)> {
    use windows::Win32::NetworkManagement::NetManagement::{FILTER_NORMAL_ACCOUNT, MAX_PREFERRED_LENGTH, NetApiBufferFree, NetUserEnum, USER_INFO_10};
    use windows::core::PCWSTR;

    let mut accounts = Vec::new();
    let (mut buf, mut read, mut total) = (std::ptr::null_mut(), 0u32, 0u32);
    // Level 10 holds the full name; asking for everything at once leaves nothing to resume.
    let status = unsafe { NetUserEnum(PCWSTR::null(), 10, FILTER_NORMAL_ACCOUNT, &mut buf, MAX_PREFERRED_LENGTH, &mut read, &mut total, None) };
    if status == 0 && !buf.is_null() {
        let users = unsafe { std::slice::from_raw_parts(buf as *const USER_INFO_10, read as usize) };
        let text = |p: windows::core::PWSTR| if p.is_null() { None } else { unsafe { p.to_string() }.ok() };
        for user in users {
            if let Some(name) = text(user.usri10_name) {
                accounts.push((name, text(user.usri10_full_name)));
            }
        }
    }
    if !buf.is_null() {
        unsafe { NetApiBufferFree(Some(buf as *const _)) };
    }
    accounts
}

/// The email addresses this PC's user signs in with: the work or school account's, the Microsoft
/// accounts Windows knows, and OneDrive's.
#[cfg(windows)]
fn account_emails() -> Vec<String> {
    use windows::Win32::Security::Authentication::Identity::NameUserPrincipal;
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let mut emails: Vec<String> = user_name_ex(NameUserPrincipal).into_iter().collect();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    // Microsoft accounts: one subkey per address.
    if let Ok(key) = hkcu.open_subkey(r"Software\Microsoft\IdentityCRL\UserExtendedProperties") {
        emails.extend(key.enum_keys().flatten());
    }
    if let Ok(accounts) = hkcu.open_subkey(r"Software\Microsoft\OneDrive\Accounts") {
        for account in accounts.enum_keys().flatten() {
            emails.extend(accounts.open_subkey(&account).ok().and_then(|k| k.get_value::<String, _>("UserEmail").ok()));
        }
    }
    emails.retain(|e| e.contains('@'));
    emails
}

/// The signed-in account's name in one of Windows' formats, when it has one in that format. A
/// domain PC away from its network can wait a long time for the domain to answer, so this gives
/// up after two seconds.
#[cfg(windows)]
fn user_name_ex(format: windows::Win32::Security::Authentication::Identity::EXTENDED_NAME_FORMAT) -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || tx.send(user_name_ex_now(format)));
    rx.recv_timeout(std::time::Duration::from_secs(2)).ok().flatten()
}

#[cfg(windows)]
fn user_name_ex_now(format: windows::Win32::Security::Authentication::Identity::EXTENDED_NAME_FORMAT) -> Option<String> {
    use windows::Win32::Security::Authentication::Identity::GetUserNameExW;
    use windows::core::PWSTR;
    let mut len = 0u32;
    // The first call fails on purpose and reports the size needed.
    let _ = unsafe { GetUserNameExW(format, None, &mut len) };
    if len == 0 {
        return None;
    }
    let mut buf = vec![0u16; len as usize];
    unsafe { GetUserNameExW(format, Some(PWSTR(buf.as_mut_ptr())), &mut len) }.then(|| String::from_utf16_lossy(&buf[..len as usize])).filter(|s| !s.trim().is_empty())
}

/// Every profile folder's account name on this PC, not just the one running the scan: disk names and
/// machine-wide startup entries can carry anyone's name. Plus the computer's names: the short
/// one in `COMPUTERNAME` (cut to 15 characters), the DNS one and the DNS one with its domain.
#[cfg(windows)]
fn machine_names() -> (Vec<String>, Vec<String>) {
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
    // Redactor drops duplicates, empty names and names too short to match safely. The full name
    // with the domain (PC01.corp.contoso.com) goes first, whole.
    use windows::Win32::System::SystemInformation::{ComputerNameDnsFullyQualified, ComputerNameDnsHostname};
    let hosts = [std::env::var("COMPUTERNAME").ok(), computer_name(ComputerNameDnsHostname), computer_name(ComputerNameDnsFullyQualified)].into_iter().flatten().collect();
    (names, hosts)
}

#[cfg(windows)]
fn computer_name(format: windows::Win32::System::SystemInformation::COMPUTER_NAME_FORMAT) -> Option<String> {
    use windows::Win32::System::SystemInformation::GetComputerNameExW;
    use windows::core::PWSTR;
    let mut len = 0u32;
    // The first call fails on purpose and reports the size needed.
    let _ = unsafe { GetComputerNameExW(format, None, &mut len) };
    let mut buf = vec![0u16; len as usize];
    unsafe { GetComputerNameExW(format, Some(PWSTR(buf.as_mut_ptr())), &mut len) }.ok()?;
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

    // An account renamed after its profile folder was made, or one that never signed in, has a name
    // no folder shows. The messages never print the names: they're this PC's.
    #[cfg(windows)]
    #[test]
    fn every_local_account_name_is_hidden() {
        let r = super::redactor();
        let accounts = super::local_accounts();
        assert!(!accounts.is_empty());
        for (name, _) in accounts {
            assert!(!r.text(&format!(r"PC\{name}")).contains(&name), "a local account's name wasn't hidden");
        }
    }

    #[cfg(windows)]
    #[test]
    fn this_pcs_full_names_and_addresses_are_hidden() {
        let r = super::redactor();
        for name in super::full_names(&super::machine_names().0, &super::local_accounts()).iter().map(|n| n.trim()).filter(|n| n.chars().count() >= 3) {
            assert!(!r.text(&format!("{name} backup")).contains(name), "a full name on this PC wasn't hidden");
        }
        for email in super::account_emails() {
            assert_eq!(r.text(&email), "<email>", "an account address on this PC wasn't hidden");
        }
    }
}
