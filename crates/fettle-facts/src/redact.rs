//! The last privacy pass over a finished report, before it is shown, saved or shared.
//!
//! The report format has no fields for serials, product keys or network details, so what is left
//! to catch is identifying text that slips into ordinary strings: the PC's name, account names,
//! the account's full name, the domain and email addresses in error details and startup names,
//! profile folders in paths, account SIDs, and (in error details only) MAC, IPv4 and IPv6 addresses and
//! network server names. The app gathers the names on Windows; this module only rewrites text.

use crate::startup_names::friendly_startup_name;
use crate::{Check, Report};

/// The current machine's names, which must not appear anywhere in a report.
#[derive(Debug, Clone, Default)]
pub struct Redactor {
    usernames: Vec<String>,
    /// Account names that are too short or too common to match as plain words ("al", "User",
    /// "Ufficio"): hidden only right after a `\` or `/`, where they name the account (`PC\al`,
    /// `/home/al`); a profile folder is hidden whatever its name.
    context_names: Vec<String>,
    hostnames: Vec<String>,
    domains: Vec<String>,
}

/// Names shorter than this are too likely to match unrelated words.
const MIN_NAME_LEN: usize = 3;

/// What this module writes in place of private text. A name is never matched inside one of these.
const PLACEHOLDERS: &[&str] = &["<user>", "<pc-name>", "<domain>", "<email>", "<server>", "<sid>", "<private>", "<mac>", "<ip>"];

/// Words a full name may hold that aren't anyone's name ("Windows User", "Utente PC", "Dell Owner"):
/// matching them would hide ordinary words in startup names.
const COMMON_WORDS: &[&str] = &[
    "user", "users", "admin", "administrator", "owner", "guest", "windows", "microsoft", "home", "office", "family", "local",
    "account", "utente", "amministratore", "proprietario", "famiglia", "casa", "ufficio", "the", "and", "del", "della", "van",
    "von", "dell", "lenovo", "acer", "asus", "msi", "toshiba", "samsung", "sony", "fujitsu", "surface", "medion", "gigabyte",
    // The name part of shared addresses ("info@…", "segreteria@…").
    "info", "mail", "email", "contact", "contacts", "support", "sales", "hello", "noreply", "contatti", "segreteria",
    "amministrazione", "ufficio", "commerciale", "assistenza",
];

/// What Windows puts in USERDOMAIN when the account isn't in a company domain.
/// Also what home routers hand out as a DNS suffix, and Entra ID's default tenant name.
const NOT_DOMAINS: &[&str] = &[
    "WORKGROUP", "AzureAD", "MicrosoftAccount", "NT AUTHORITY", "BUILTIN", "NT SERVICE", "Window Manager", "Font Driver Host",
    "lan", "local", "localdomain", "home.arpa", "fritz.box", "station", "Default Directory",
];

/// Profile folders that belong to no one in particular.
const SHARED_PROFILES: &[&str] = &["Public", "Default", "Default User", "All Users", "<user>"];

impl Redactor {
    pub fn new<S: Into<String>>(usernames: impl IntoIterator<Item = S>, hostname: impl Into<String>) -> Redactor {
        let mut r = Redactor::default();
        for name in usernames.into_iter().map(Into::into) {
            let name = name.trim().to_string();
            if name.chars().count() >= MIN_NAME_LEN && !is_common_word(&name) {
                add_name(&mut r.usernames, name);
            } else if !name.is_empty() && !r.context_names.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
                r.context_names.push(name);
            }
        }
        r.with_hostname(hostname)
    }

    /// The account's full name ("Mario Rossi"): hidden whole, and by each of its words that isn't a
    /// common one, so a disk renamed "Rossi backup" doesn't give the surname away.
    pub fn with_full_name(mut self, name: impl Into<String>) -> Redactor {
        let name = name.into().trim().to_string();
        // "Rossi, Mario (Contoso)": words end at anything that can't be part of a name.
        let words = name.split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '-')).filter(|w| !w.is_empty());
        for word in words.filter(|w| !is_common_word(w)) {
            if word.chars().count() >= MIN_NAME_LEN {
                add_name(&mut self.usernames, word.to_string());
            }
        }
        if name.chars().count() >= MIN_NAME_LEN {
            add_name(&mut self.usernames, name);
        }
        self
    }

    /// An address the account signs in with. The address itself is hidden wherever it appears;
    /// its name part ("mario.rossi") is hidden on its own too, unless it's a shared word ("info@…").
    pub fn with_email(mut self, email: impl Into<String>) -> Redactor {
        let email = email.into();
        if let Some((name, _)) = email.trim().split_once('@')
            && name.chars().count() >= MIN_NAME_LEN
            && !is_common_word(name)
        {
            add_name(&mut self.usernames, name.to_string());
        }
        self
    }

    /// The account's domain (USERDOMAIN): a company's name. Outside a domain Windows puts the PC's
    /// name or a fixed word there, which aren't a domain.
    pub fn with_domain(mut self, domain: impl Into<String>) -> Redactor {
        let domain = domain.into().trim().to_string();
        let usual = NOT_DOMAINS.iter().any(|d| d.eq_ignore_ascii_case(&domain)) || is_common_word(&domain) || self.hostnames.iter().any(|h| h.eq_ignore_ascii_case(&domain));
        if domain.chars().count() >= MIN_NAME_LEN && !usual {
            add_name(&mut self.domains, domain);
        }
        self
    }

    /// Another name this PC goes by: `COMPUTERNAME` is cut to 15 characters, the DNS name isn't.
    pub fn with_hostname(mut self, hostname: impl Into<String>) -> Redactor {
        let host = hostname.into().trim().to_string();
        if !host.is_empty() && !self.hostnames.iter().any(|h| h.eq_ignore_ascii_case(&host)) {
            self.hostnames.push(host);
            // Longest first, so the full name goes before its short form can match inside it.
            self.hostnames.sort_by_key(|h| std::cmp::Reverse(h.len()));
        }
        self
    }

    /// Text from firmware and drivers (CPU, board, disk and GPU names, the Windows edition): only
    /// structural identifiers go. Names aren't matched here, because they can't appear in such text
    /// but can collide with its words (a user called "Pro" and "Windows 11 Pro").
    pub fn hardware(&self, s: &str) -> String {
        let s = replace_account_sids(s);
        replace_profile_folders(&hide_usbstor_instance(&s))
    }

    /// Free text (startup names, error details) with the PC's identity taken out.
    pub fn text(&self, s: &str) -> String {
        // Email addresses first, whole, before a name inside one ("mario.rossi@…") can match.
        let mut s = replace_emails(&self.hardware(s));
        // PC names first: they often start with the username ("MARIO-PC"), and replacing the
        // username first would leave the rest of the PC name behind.
        for host in &self.hostnames {
            s = replace_word(&s, host, "<pc-name>");
        }
        for domain in &self.domains {
            s = replace_word(&s, domain, "<domain>");
        }
        for name in &self.usernames {
            s = replace_word(&s, name, "<user>");
        }
        for name in &self.context_names {
            s = replace_after_separator(&s, name, "<user>");
        }
        s
    }

    /// Free text such as error details, which may also quote network addresses and servers.
    pub fn detail(&self, s: &str) -> String {
        // IPv4 before IPv6, so an IPv4-mapped address (::ffff:192.168.1.10) goes whole.
        replace_unc_servers(&replace_ipv6(&replace_ipv4(&replace_macs(&self.text(s)))))
    }

    /// Rewrites every text field in the report, and gives startup items their readable names.
    /// Dates, the volume letter and the app's own fields are left alone: they can't carry a name.
    pub fn report(&self, report: &mut Report) {
        let text = |v: &mut Option<String>| {
            if let Some(s) = v {
                *s = self.hardware(s);
            }
        };
        let detail = |check: &mut Check| {
            if let Some(s) = &mut check.detail {
                *s = self.detail(s);
            }
        };
        let facts = &mut report.facts;
        if let Some(sec) = &mut facts.system {
            detail(&mut sec.check);
            if let Some(d) = &mut sec.data {
                for v in [&mut d.cpu_name, &mut d.board, &mut d.bios_version] {
                    text(v);
                }
                d.displays.iter_mut().for_each(|x| text(&mut x.name));
            }
        }
        if let Some(sec) = &mut facts.storage {
            detail(&mut sec.check);
            if let Some(d) = &mut sec.data {
                // Disk names can be renamed by the user (Storage Spaces), so they count as free text.
                for disk in &mut d.disks {
                    if let Some(model) = &mut disk.model {
                        *model = self.text(model);
                    }
                }
            }
        }
        if let Some(sec) = &mut facts.battery {
            detail(&mut sec.check);
        }
        if let Some(sec) = &mut facts.memory {
            detail(&mut sec.check);
            if let Some(d) = &mut sec.data {
                for m in &mut d.modules {
                    for v in [&mut m.part_number, &mut m.manufacturer, &mut m.bank_label, &mut m.device_locator] {
                        text(v);
                    }
                }
            }
        }
        if let Some(sec) = &mut facts.windows {
            detail(&mut sec.check);
            if let Some(d) = &mut sec.data {
                for v in [&mut d.product, &mut d.edition_id, &mut d.display_version, &mut d.installation_type] {
                    text(v);
                }
                if let Some(tpm) = &mut d.tpm {
                    text(&mut tpm.spec_version);
                }
            }
        }
        if let Some(sec) = &mut facts.startup {
            detail(&mut sec.check);
            if let Some(d) = &mut sec.data {
                for item in &mut d.items {
                    if let Some(name) = &mut item.name {
                        *name = self.text(&friendly_startup_name(name));
                    }
                }
            }
        }
        if let Some(sec) = &mut facts.graphics {
            detail(&mut sec.check);
            if let Some(d) = &mut sec.data {
                for g in &mut d.adapters {
                    text(&mut g.name);
                    text(&mut g.driver_version);
                }
            }
        }
    }
}

/// Adds `name` unless it's there already, keeping the longest first, so "jdoe.CONTOSO" is replaced
/// whole before "jdoe" can match inside it.
fn add_name(names: &mut Vec<String>, name: String) {
    if !names.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
        names.push(name);
        names.sort_by_key(|n| std::cmp::Reverse(n.len()));
    }
}

fn is_common_word(word: &str) -> bool {
    COMMON_WORDS.iter().any(|c| c.eq_ignore_ascii_case(word))
}

fn is_shared_profile(name: &str) -> bool {
    SHARED_PROFILES.iter().any(|p| p.eq_ignore_ascii_case(name))
}

/// Local and domain account SIDs (S-1-5-21-…) and Entra ID ones (S-1-12-1-…) identify this PC and
/// its users, in any letter case. Well-known SIDs like S-1-5-18 are left alone.
fn replace_account_sids(s: &str) -> String {
    const PREFIXES: [&str; 2] = ["s-1-5-21-", "s-1-12-1-"];
    // ASCII lowercasing keeps byte offsets identical to `s`.
    let lower = s.to_ascii_lowercase();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while let Some((pos, prefix)) = PREFIXES.iter().filter_map(|p| lower[i..].find(p).map(|at| (i + at, p))).min() {
        out.push_str(&s[i..pos]);
        let tail = &s[pos + prefix.len()..];
        let len = tail.find(|c: char| !(c.is_ascii_digit() || c == '-')).unwrap_or(tail.len());
        out.push_str("<sid>");
        i = pos + prefix.len() + len;
    }
    out.push_str(&s[i..]);
    out
}

/// USB disk device IDs end with the device's serial: USBSTOR\<model>\<serial>&0.
fn hide_usbstor_instance(s: &str) -> String {
    let is_usbstor = s.get(..8).is_some_and(|p| p.eq_ignore_ascii_case(r"USBSTOR\"));
    match s.rfind('\\') {
        Some(pos) if is_usbstor && pos > 7 => format!("{}<private>", &s[..=pos]),
        _ => s.to_string(),
    }
}

/// Replaces the folder name after any `\Users\` (or `/Users/`), so profiles of other or renamed
/// accounts don't leak even when their name isn't known.
fn replace_profile_folders(s: &str) -> String {
    const MARKERS: [&str; 2] = [r"\users\", "/users/"];
    // ASCII lowercasing keeps byte offsets identical to `s`.
    let lower = s.to_ascii_lowercase();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while let Some(found) = MARKERS.iter().filter_map(|m| lower[i..].find(m)).min() {
        let seg_start = i + found + MARKERS[0].len();
        let rest = &s[seg_start..];
        // Folder names can hold spaces ("Ana Lima"), so the name ends at a separator, a line break,
        // or where an error message carries on (" (os error 5)", ": access denied"). Brackets alone
        // don't end it: "Ana (Lima)" is a valid account name.
        let ends = [rest.find(['\\', '/', '"', '\'', '\n', '\r']), rest.find(" (os error"), rest.find(": ")];
        let seg_len = ends.into_iter().flatten().min().unwrap_or(rest.len());
        let segment = &s[seg_start..seg_start + seg_len];
        out.push_str(&s[i..seg_start]);
        out.push_str(if segment.is_empty() || is_shared_profile(segment) { segment } else { "<user>" });
        i = seg_start + seg_len;
    }
    out.push_str(&s[i..]);
    out
}

fn is_word_char(c: Option<char>) -> bool {
    c.is_some_and(char::is_alphanumeric)
}

/// Rewrites each match of `matcher`, which returns the byte length matched at a position.
fn replace_matches(s: &str, with: &str, matcher: impl Fn(&str, usize) -> Option<usize>) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if let Some(len) = matcher(s, i) {
            out.push_str(with);
            i += len;
        } else {
            let c = s[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

/// True when byte `i` falls inside a placeholder this module wrote.
fn inside_placeholder(s: &str, i: usize) -> bool {
    let b = s.as_bytes();
    PLACEHOLDERS.iter().any(|p| {
        let p = p.as_bytes();
        (i.saturating_sub(p.len() - 1)..=i).any(|start| b.get(start..start + p.len()) == Some(p))
    })
}

/// The byte length of `needle` at byte `i` of `s`, ignoring letter case in any alphabet ("JOSÉ" is
/// "josé"), or None when it isn't there. Letter by letter: the few letters whose other case is two
/// letters (Turkish "İ", German "ß" as "SS") only match in the case they were written in.
fn match_ignoring_case(s: &str, i: usize, needle: &str) -> Option<usize> {
    let mut hay = s[i..].char_indices();
    for n in needle.chars() {
        let (_, h) = hay.next()?;
        if h != n && !h.to_lowercase().eq(n.to_lowercase()) {
            return None;
        }
    }
    Some(hay.next().map_or(s.len() - i, |(at, _)| at))
}

/// `needle` (in any letter case) where it stands as a whole word, skipping placeholders.
fn replace_word(s: &str, needle: &str, with: &str) -> String {
    if needle.chars().count() < MIN_NAME_LEN {
        return s.to_string();
    }
    replace_matches(s, with, |s, i| {
        let len = match_ignoring_case(s, i, needle)?;
        let found = !is_word_char(s[..i].chars().next_back()) && !is_word_char(s[i + len..].chars().next()) && !inside_placeholder(s, i);
        found.then_some(len)
    })
}

/// `name` (in any letter case) as a whole word right after a `\` or `/`.
fn replace_after_separator(s: &str, name: &str, with: &str) -> String {
    replace_matches(s, with, |s, i| {
        let len = match_ignoring_case(s, i, name)?;
        let found = matches!(s[..i].chars().next_back(), Some('\\' | '/')) && !is_word_char(s[i + len..].chars().next());
        found.then_some(len)
    })
}

/// Email addresses: a name, an @ and a domain with at least one dot that ends in letters.
fn replace_emails(s: &str) -> String {
    let local = |c: u8| c.is_ascii_alphanumeric() || b"._%+_-'".contains(&c);
    replace_matches(s, "<email>", |s, i| {
        let b = s.as_bytes();
        if i > 0 && local(b[i - 1]) {
            return None;
        }
        let name = b[i..].iter().take_while(|c| local(**c)).count();
        if name == 0 || b.get(i + name) != Some(&b'@') {
            return None;
        }
        let at = i + name + 1;
        let run = b[at..].iter().take_while(|c| c.is_ascii_alphanumeric() || **c == b'-' || **c == b'.').count();
        // A sentence can end right after an address: "for ana@example.com."
        let domain = s[at..at + run].trim_end_matches('.');
        let labels: Vec<&str> = domain.split('.').collect();
        let top = labels.last().copied().unwrap_or("");
        let ok = labels.len() >= 2 && labels.iter().all(|l| !l.is_empty()) && top.len() >= 2 && top.bytes().all(|c| c.is_ascii_alphabetic());
        ok.then_some(name + 1 + domain.len())
    })
}

/// The server in a network path (`\\NAS\share` and the long form `\\?\UNC\NAS\share` →
/// `<server>`). Windows' own `\\?\` and `\\.\` prefixes aren't servers.
fn replace_unc_servers(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find(r"\\") {
        let (before, mut after) = rest.split_at(at + 2);
        out.push_str(before);
        const LONG_FORM: &str = r"?\UNC\";
        if let Some(long) = after.get(..LONG_FORM.len()).filter(|p| p.eq_ignore_ascii_case(LONG_FORM)) {
            out.push_str(long);
            after = &after[LONG_FORM.len()..];
        }
        let len: usize = after.chars().take_while(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.')).map(char::len_utf8).sum();
        let server = &after[..len];
        if server.chars().any(char::is_alphanumeric) {
            out.push_str("<server>");
            rest = &after[len..];
        } else {
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// IPv6 addresses: hex groups of up to four digits joined by ':', either all eight or shortened with
/// "::", with an optional zone ("%12"). Times ("10:20:30") and "std::io" don't qualify.
fn replace_ipv6(s: &str) -> String {
    replace_matches(s, "<ip>", |s, i| {
        let b = s.as_bytes();
        let edge = |c: Option<&u8>| c.is_some_and(|c| c.is_ascii_alphanumeric() || *c == b':' || *c == b'.');
        if edge(i.checked_sub(1).and_then(|j| b.get(j))) || !(b[i].is_ascii_hexdigit() || b[i] == b':') {
            return None;
        }
        let run = b[i..].iter().take_while(|c| c.is_ascii_hexdigit() || **c == b':').count();
        let addr = &s[i..i + run];
        let groups: Vec<&str> = addr.split(':').collect();
        let colons = groups.len() - 1;
        let shape = addr.contains("::") && !addr.contains(":::") || colons == 7;
        let ok = shape && colons >= 2 && colons <= 7 && groups.iter().all(|g| g.len() <= 4) && addr.bytes().any(|c| c.is_ascii_digit());
        if !ok {
            return None;
        }
        // A zone ("%12", "%eth0") belongs to the address.
        let zone = if b.get(i + run) == Some(&b'%') { 1 + b[i + run + 1..].iter().take_while(|c| c.is_ascii_alphanumeric()).count() } else { 0 };
        let end = i + run + zone;
        (!b.get(end).is_some_and(u8::is_ascii_alphanumeric)).then_some(end - i)
    })
}

/// Six hex pairs joined by ':' or '-', the same one throughout.
fn replace_macs(s: &str) -> String {
    replace_matches(s, "<mac>", |s, i| {
        let b = s.as_bytes();
        let cand = b.get(i..i + 17)?;
        let sep = cand[2];
        let pairs = (sep == b':' || sep == b'-')
            && cand.chunks(3).all(|c| c[0].is_ascii_hexdigit() && c[1].is_ascii_hexdigit() && (c.len() == 2 || c[2] == sep));
        let edge = |c: Option<&u8>| c.is_some_and(|c| c.is_ascii_alphanumeric() || *c == sep);
        (pairs && !edge(i.checked_sub(1).and_then(|j| b.get(j))) && !edge(b.get(i + 17))).then_some(17)
    })
}

/// Four dotted numbers from 0 to 255, not part of a longer dotted run such as a version number.
fn replace_ipv4(s: &str) -> String {
    replace_matches(s, "<ip>", |s, i| {
        let b = s.as_bytes();
        let before = i.checked_sub(1).and_then(|j| b.get(j));
        if !b[i].is_ascii_digit() || before.is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'.') {
            return None;
        }
        let run = b[i..].iter().take_while(|c| c.is_ascii_digit() || **c == b'.').count();
        // A sentence can end right after an address: "at 10.0.0.7."
        let len = if b[i + run - 1] == b'.' { run - 1 } else { run };
        let parts: Vec<&str> = s[i..i + len].split('.').collect();
        let octets = parts.len() == 4 && parts.iter().all(|p| (1..=3).contains(&p.len()) && p.parse::<u16>().is_ok_and(|n| n <= 255));
        (octets && !b.get(i + len).is_some_and(u8::is_ascii_alphabetic)).then_some(len)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CheckStatus, Disk, Facts, Section, Startup, StartupItem, Storage};

    fn r() -> Redactor {
        Redactor::new(["alex", "jdoe"], "DESKTOP-ABC")
    }

    #[test]
    fn account_and_pc_names_are_replaced_case_insensitively() {
        assert_eq!(r().text(r"DESKTOP-ABC\jdoe"), r"<pc-name>\<user>");
        assert_eq!(r().text("signed in as ALEX on desktop-abc"), "signed in as <user> on <pc-name>");
    }

    #[test]
    fn names_only_match_whole_words() {
        let r = Redactor::new(["dan"], "PC");
        assert_eq!(r.text("Guardian Dan"), "Guardian <user>");
        assert_eq!(r.text("Danfoss"), "Danfoss");
    }

    #[test]
    fn short_or_empty_names_are_ignored() {
        let r = Redactor::new(["al", ""], "");
        assert_eq!(r.text("Realtek PCIe"), "Realtek PCIe");
    }

    #[test]
    fn any_profile_folder_is_replaced_even_when_the_name_is_unknown() {
        assert_eq!(r().text(r"C:\Users\mystery\AppData\x.exe"), r"C:\Users\<user>\AppData\x.exe");
        assert_eq!(r().text(r"c:\users\Ana Lima\x"), r"c:\users\<user>\x");
        assert_eq!(r().text(r"C:\Users\Public\Desktop"), r"C:\Users\Public\Desktop");
    }

    #[test]
    fn account_sids_are_replaced_and_well_known_ones_kept() {
        assert_eq!(r().text(r"HKU\S-1-5-21-111111111-2222222222-333333333-1001\Run"), r"HKU\<sid>\Run");
        assert_eq!(r().text("S-1-5-18"), "S-1-5-18");
    }

    #[test]
    fn usb_disk_serials_are_hidden() {
        assert_eq!(
            r().text(r"USBSTOR\DISK&VEN_SANDISK&PROD_ULTRA&REV_1.00\4C530001230101115&0"),
            r"USBSTOR\DISK&VEN_SANDISK&PROD_ULTRA&REV_1.00\<private>"
        );
    }

    #[test]
    fn details_also_hide_mac_and_ipv4_addresses() {
        assert_eq!(r().detail("adapter 00:1A:2B:3C:4D:5E at 192.168.1.20 failed"), "adapter <mac> at <ip> failed");
        assert_eq!(r().detail("mac 00-1a-2b-3c-4d-5e"), "mac <mac>");
    }

    #[test]
    fn version_numbers_are_not_mistaken_for_addresses() {
        assert_eq!(r().detail("driver 32.0.15.6094, build 10.0.26100.1"), "driver 32.0.15.6094, build 10.0.26100.1");
        assert_eq!(r().detail("1.2.3.4.5"), "1.2.3.4.5");
        // Ordinary strings keep dotted numbers: driver versions like 1.0.0.1 are real data.
        assert_eq!(r().text("1.0.0.1"), "1.0.0.1");
    }

    #[test]
    fn names_in_angle_brackets_are_still_hidden() {
        // The brackets were in the original text; only the name inside is replaced.
        assert_eq!(r().text("user <jdoe> not found"), "user <<user>> not found");
        // ...while placeholders stay intact, even for a user called "name" or "user".
        let r = Redactor::new(["name", "user"], "PC-NAME");
        assert_eq!(r.text("<pc-name> <user>"), "<pc-name> <user>");
    }

    #[test]
    fn entra_and_lowercase_sids_are_replaced() {
        assert_eq!(r().text(r"HKU\S-1-12-1-1111111111-2222222222-3333333333-4444444444\Run"), r"HKU\<sid>\Run");
        assert_eq!(r().text("s-1-5-21-1-2-3-1001"), "<sid>");
    }

    #[test]
    fn a_long_pc_name_is_hidden_whole_and_by_its_short_form() {
        let r = Redactor::new(["jdoe"], "MARIOS-GAMING-R").with_hostname("MARIOS-GAMING-RIG");
        assert_eq!(r.text("MARIOS-GAMING-RIG and MARIOS-GAMING-R"), "<pc-name> and <pc-name>");
    }

    #[test]
    fn a_profile_folder_ends_before_the_rest_of_an_error() {
        // Account names can hold brackets: the whole folder goes.
        assert_eq!(r().text(r"C:\Users\Ana (Lima)\x"), r"C:\Users\<user>\x");
        assert_eq!(r().text(r"open C:\Users\Ana (Lima) (os error 5)"), r"open C:\Users\<user> (os error 5)");
        assert_eq!(r().text(r"can't read C:\Users\alex (os error 5)"), r"can't read C:\Users\<user> (os error 5)");
        assert_eq!(r().text(r"C:\Users\Ana Lima: access denied"), r"C:\Users\<user>: access denied");
        assert_eq!(r().text("C:\\Users\\alex\nnext line"), "C:\\Users\\<user>\nnext line");
    }

    /// Every text field, filled with private text, comes out clean (except the fields that can't
    /// carry a name: the app's own, the dates and the volume letter).
    #[test]
    fn every_text_field_goes_through_the_pass() {
        let private = r"S-1-5-21-1-2-3-1001 C:\Users\mystery\x";
        let section = |data: serde_json::Value| serde_json::json!({ "check": { "status": "failed", "detail": private }, "data": data });
        let json = serde_json::json!({
            "schema_version": 1, "app_version": "0.1.0", "collected_at": "2026-10-07T21:00:00Z", "elevated": false,
            "facts": {
                "system": section(serde_json::json!({ "cpu_name": private, "board": private, "bios_version": private, "displays": [{ "name": private }] })),
                "storage": section(serde_json::json!({ "disks": [{ "model": private }], "volumes": [{ "letter": "C:" }] })),
                "battery": section(serde_json::json!({})),
                "memory": section(serde_json::json!({ "modules": [{ "part_number": private, "manufacturer": private, "bank_label": private, "device_locator": private }] })),
                "windows": section(serde_json::json!({ "product": private, "edition_id": private, "display_version": private, "installation_type": private, "tpm": { "spec_version": private } })),
                "startup": section(serde_json::json!({ "items": [{ "name": private }] })),
                "graphics": section(serde_json::json!({ "adapters": [{ "name": private, "driver_version": private }] })),
            }
        });
        let mut report: Report = serde_json::from_value(json).unwrap();
        r().report(&mut report);
        let out = serde_json::to_string(&report).unwrap();
        assert!(!out.contains("S-1-5-21") && !out.contains("mystery"), "{out}");
        assert_eq!(out.matches("<sid>").count(), 24, "one per field: 7 details + 17 data fields");
    }

    #[test]
    fn a_pc_name_that_starts_with_the_username_is_hidden_whole() {
        let r = Redactor::new(["mario"], "MARIO-PC");
        assert_eq!(r.text(r"MARIO-PC\mario on MARIO-PC"), r"<pc-name>\<user> on <pc-name>");
    }

    #[test]
    fn edge_cases_stay_readable() {
        // A user called "user" eats neither the placeholders nor the word, and "jdoe.CONTOSO" goes whole.
        let r = Redactor::new(["user", "jdoe", "jdoe.CONTOSO"], "DESKTOP-ABC");
        assert_eq!(r.text(r"C:\Users\jdoe.CONTOSO\x"), r"C:\Users\<user>\x");
        assert_eq!(r.text("user jdoe.CONTOSO <user>"), "user <user> <user>");
        // Accented text around a name, and an address ending a sentence.
        assert_eq!(r.text("Caffè di jdoe, perché sì"), "Caffè di <user>, perché sì");
        assert_eq!(r.detail("non raggiungibile: 10.0.0.7."), "non raggiungibile: <ip>.");
        assert_eq!(r.detail("è 00:1A:2B:3C:4D:5E"), "è <mac>");
    }

    #[test]
    fn email_addresses_are_hidden() {
        assert_eq!(r().text("OneDrive - mario.rossi@contoso.com"), "OneDrive - <email>");
        assert_eq!(r().detail("sign-in failed for Ana_Lima+fettle@example.co.uk."), "sign-in failed for <email>.");
        // Not an address: no dot after the @, or nothing before it.
        assert_eq!(r().text("Tool @home and x@y"), "Tool @home and x@y");
    }

    #[test]
    fn a_profile_path_with_forward_slashes_is_hidden() {
        assert_eq!(r().text("C:/Users/mystery/AppData/x.exe"), "C:/Users/<user>/AppData/x.exe");
        assert_eq!(r().text("C:/Users/Public/x"), "C:/Users/Public/x");
    }

    #[test]
    fn a_network_server_name_in_an_error_is_hidden() {
        assert_eq!(r().detail(r"can't open \\NAS-ROSSI\backup\x"), r"can't open \\<server>\backup\x");
        assert_eq!(r().detail(r"\\?\C:\long\path"), r"\\?\C:\long\path");
        assert_eq!(r().detail(r"\\.\PhysicalDrive0"), r"\\.\PhysicalDrive0");
    }

    #[test]
    fn a_short_account_name_is_hidden_where_it_names_the_account() {
        // Two letters would match too many words, so only after a separator, as in PC\al or /al/.
        let r = Redactor::new(["al"], "DESKTOP-ABC");
        assert_eq!(r.text(r"DESKTOP-ABC\al"), r"<pc-name>\<user>");
        assert_eq!(r.text("/home/al/x"), "/home/<user>/x");
        assert_eq!(r.text("Realtek al Alarm"), "Realtek al Alarm");
    }

    #[test]
    fn the_full_name_is_hidden_whole_and_by_its_words() {
        let r = Redactor::new(["mario"], "DESKTOP-ABC").with_full_name("Mario Rossi");
        assert_eq!(r.text("Mario Rossi.lnk"), "<user>.lnk");
        assert_eq!(r.text("Rossi backup"), "<user> backup");
        // Common words in a full name ("Windows User", "Admin") aren't names.
        let r = Redactor::new(["mario"], "DESKTOP-ABC").with_full_name("Windows User");
        assert_eq!(r.text("Windows Security notification icon"), "Windows Security notification icon");
        assert_eq!(r.text("Windows User Sync"), "<user> Sync");
    }

    #[test]
    fn the_domain_is_hidden_but_not_the_usual_placeholders() {
        let r = Redactor::new(["jdoe"], "DESKTOP-ABC").with_domain("CONTOSO");
        assert_eq!(r.text(r"CONTOSO\jdoe"), r"<domain>\<user>");
        for usual in ["WORKGROUP", "AzureAD", "MicrosoftAccount", "NT AUTHORITY", "BUILTIN"] {
            let r = Redactor::new(["jdoe"], "DESKTOP-ABC").with_domain(usual);
            assert_eq!(r.text(usual), usual);
        }
        // On a PC outside a domain, the domain is the PC's own name.
        let r = Redactor::new(["jdoe"], "DESKTOP-ABC").with_domain("DESKTOP-ABC");
        assert_eq!(r.text("DESKTOP-ABC"), "<pc-name>");
    }

    #[test]
    fn a_full_name_written_surname_first_is_hidden_by_its_words() {
        let r = Redactor::new(["mario"], "DESKTOP-ABC").with_full_name("Rossi, Mario (Contoso)");
        assert_eq!(r.text("Rossi backup"), "<user> backup");
        assert_eq!(r.text("Contoso Sync"), "<user> Sync");
    }

    #[test]
    fn long_form_and_unusual_network_paths_are_hidden() {
        assert_eq!(r().detail(r"open \\?\UNC\NAS-ROSSI\share\x"), r"open \\?\UNC\<server>\share\x");
        assert_eq!(r().detail(r"can't reach \\NAS-ROSSI"), r"can't reach \\<server>");
        assert_eq!(r().detail(r"\\SERVÈR\x"), r"\\<server>\x");
        // A web address isn't a network path.
        assert_eq!(r().detail("https://fettle.page/x"), "https://fettle.page/x");
    }

    #[test]
    fn a_shared_address_doesn_t_hide_ordinary_words() {
        let r = Redactor::new(["mario"], "DESKTOP-ABC").with_email("info@studio.example").with_email("office@example.com").with_email("mario.rossi@example.com");
        assert_eq!(r.text("Microsoft Office info"), "Microsoft Office info");
        assert_eq!(r.text("mario.rossi backup"), "<user> backup");
        assert_eq!(r.text("info@studio.example"), "<email>");
    }

    #[test]
    fn an_address_with_an_apostrophe_is_hidden_whole() {
        assert_eq!(r().text("o'brien@example.com"), "<email>");
    }

    #[test]
    fn an_account_named_with_a_common_word_is_hidden_only_where_it_names_the_account() {
        // Accounts called "User", "Admin" or "Ufficio" would garble ordinary words everywhere else.
        let r = Redactor::new(["User", "Ufficio"], "DESKTOP-ABC");
        assert_eq!(r.text("Dell SupportAssist User Agent"), "Dell SupportAssist User Agent");
        assert_eq!(r.detail("The user name or password is incorrect."), "The user name or password is incorrect.");
        assert_eq!(r.text(r"DESKTOP-ABC\User"), r"<pc-name>\<user>");
        assert_eq!(r.text(r"C:\Users\Ufficio\AppData"), r"C:\Users\<user>\AppData");
    }

    #[test]
    fn names_with_accents_are_hidden_in_any_case() {
        let r = Redactor::new(["José"], "DESKTOP-ABC").with_full_name("José Nuñez");
        assert_eq!(r.text("JOSÉ NUÑEZ backup"), "<user> backup");
        assert_eq!(r.text("Nuñez e josé"), "<user> e <user>");
        // Same letters but a different word: left alone.
        assert_eq!(r.text("Joséphine"), "Joséphine");
    }

    #[test]
    fn ipv6_addresses_in_details_are_hidden() {
        assert_eq!(r().detail("no route to fe80::1c2b:3d4e:5f60:7a8b%12 now"), "no route to <ip> now");
        assert_eq!(r().detail("server 2001:db8:85a3:0:0:8a2e:370:7334."), "server <ip>.");
        // Times and ordinary colons stay.
        assert_eq!(r().detail("at 10:20:30, std::io failed: code 5"), "at 10:20:30, std::io failed: code 5");
    }

    #[test]
    fn the_pcs_full_dns_name_and_the_company_tenant_are_hidden() {
        let r = Redactor::new(["jdoe"], "PC01").with_hostname("PC01.corp.contoso-group.com").with_domain("corp.contoso-group.com").with_domain("Contoso Ltd");
        assert_eq!(r.detail("can't reach PC01.corp.contoso-group.com"), "can't reach <pc-name>");
        assert_eq!(r.detail("dc1.corp.contoso-group.com refused"), "dc1.<domain> refused");
        assert_eq!(r.text("OneDrive - Contoso Ltd"), "OneDrive - <domain>");
    }

    #[test]
    fn an_ipv4_mapped_ipv6_address_is_hidden_whole() {
        assert_eq!(r().detail("from ::ffff:192.168.1.10 refused"), "from ::ffff:<ip> refused");
    }

    #[test]
    fn ordinary_words_are_not_taken_for_a_domain() {
        // A router's DNS suffix ("lan", "home") or a tenant called "Microsoft" or "Default Directory".
        let r = Redactor::new(["jdoe"], "DESKTOP-ABC").with_domain("lan").with_domain("home").with_domain("Microsoft").with_domain("Default Directory");
        assert_eq!(r.detail("LAN adapter disconnected"), "LAN adapter disconnected");
        assert_eq!(r.text("Microsoft Edge"), "Microsoft Edge");
        assert_eq!(r.text("Default Directory sync"), "Default Directory sync");
    }

    /// Every text field in the report is either cleaned by `Redactor::report` or can't hold a
    /// name (app_version, engine_version, collected_at, Volume.letter and the three dates).
    #[test]
    fn a_new_text_field_needs_a_decision_here() {
        let lib = include_str!("lib.rs");
        let fields = lib.lines().filter(|l| l.trim_start().starts_with("pub ") && l.contains("String")).count();
        assert_eq!(fields, 25, "a text field was added or removed: decide whether Redactor::report cleans it, then update this count");
    }

    #[test]
    fn hardware_text_keeps_its_words_even_when_a_name_matches() {
        use crate::{Gpu, Graphics, System, Windows};
        let r = Redactor::new(["Pro", "Radeon"], "RYZEN");
        let mut report = Report {
            schema_version: 1,
            app_version: "0.1.0".into(),
            engine_version: None,
            collected_at: "2026-10-07T21:00:00Z".into(),
            elevated: false,
            facts: Facts {
                system: Some(Section {
                    check: Check { status: CheckStatus::Failed, detail: Some("RYZEN said no to Pro".into()) },
                    data: Some(System { cpu_name: Some("AMD Ryzen 7 9800X3D".into()), ..Default::default() }),
                }),
                windows: Some(Section { check: Check { status: CheckStatus::Ok, detail: None }, data: Some(Windows { product: Some("Windows 11 Pro".into()), ..Default::default() }) }),
                graphics: Some(Section {
                    check: Check { status: CheckStatus::Ok, detail: None },
                    data: Some(Graphics { adapters: vec![Gpu { name: Some("AMD Radeon RX 6900 XT".into()), ..Default::default() }] }),
                }),
                startup: Some(Section {
                    check: Check { status: CheckStatus::Ok, detail: None },
                    data: Some(Startup { items: vec![StartupItem { name: Some("Pro tools".into()), ..Default::default() }] }),
                }),
                ..Default::default()
            },
        };
        r.report(&mut report);
        let f = report.facts;
        let system = f.system.unwrap();
        assert_eq!(system.data.unwrap().cpu_name.as_deref(), Some("AMD Ryzen 7 9800X3D"));
        assert_eq!(f.windows.unwrap().data.unwrap().product.as_deref(), Some("Windows 11 Pro"));
        assert_eq!(f.graphics.unwrap().data.unwrap().adapters[0].name.as_deref(), Some("AMD Radeon RX 6900 XT"));
        // Free text still hides them.
        assert_eq!(system.check.detail.as_deref(), Some("<pc-name> said no to <user>"));
        assert_eq!(f.startup.unwrap().data.unwrap().items[0].name.as_deref(), Some("<user> tools"));
    }

    #[test]
    fn the_whole_report_is_cleaned() {
        let mut report = Report {
            schema_version: 1,
            app_version: "0.1.0".into(),
            engine_version: None,
            collected_at: "2026-10-07T21:00:00Z".into(),
            elevated: false,
            facts: Facts {
                storage: Some(Section {
                    check: Check { status: CheckStatus::Failed, detail: Some(r"open C:\Users\alex\x at 10.0.0.7 failed".into()) },
                    data: Some(Storage { disks: vec![Disk { model: Some(r"DESKTOP-ABC data at C:\Users\alex\vhd".into()), ..Default::default() }], ..Default::default() }),
                }),
                startup: Some(Section {
                    check: Check { status: CheckStatus::Ok, detail: None },
                    data: Some(Startup {
                        items: vec![
                            StartupItem { name: Some("MicrosoftEdgeAutoLaunch_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into()), ..Default::default() },
                            StartupItem { name: Some("jdoe's sync".into()), ..Default::default() },
                        ],
                    }),
                }),
                ..Default::default()
            },
        };
        r().report(&mut report);
        let storage = report.facts.storage.unwrap();
        assert_eq!(storage.check.detail.as_deref(), Some(r"open C:\Users\<user>\x at <ip> failed"));
        // Disk names can be renamed by the user (Storage Spaces), so they're matched for names too.
        assert_eq!(storage.data.unwrap().disks[0].model.as_deref(), Some(r"<pc-name> data at C:\Users\<user>\vhd"));
        let names: Vec<_> = report.facts.startup.unwrap().data.unwrap().items.into_iter().map(|i| i.name.unwrap()).collect();
        assert_eq!(names, ["Microsoft Edge", "<user>'s sync"]);
        assert_eq!(report.collected_at, "2026-10-07T21:00:00Z");
    }
}
