// Windows support end dates, copied from Microsoft's lifecycle pages on 2026-10-07 (LTSC 2024, LTSB 2015/2016,
// IoT LTSC 2019 and IoT Enterprise added from them on 2026-10-11).
// Each date is Microsoft's "End Date" as an instant (they list e.g. "10/14/2026 6:59:59 AM",
// which is the last second of 13 Oct in Redmond). Update with each Fettle release.

use crate::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EditionClass {
    HomePro,
    Enterprise,
    Ltsc,
    IotLtsc,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Esu {
    Consumer,
    Commercial,
    None,
}

/// `esu_end` is always enforced; `show_esu_end` says whether the date is exact enough to print.
/// `fixed`: LTSC/LTSB (the Fixed policy), where a feature update isn't the way to stay supported.
#[derive(Clone, Copy, Debug)]
pub struct SupportInfo {
    pub end: i64,
    pub esu: Esu,
    pub esu_end: Option<i64>,
    pub show_esu_end: bool,
    pub fixed: bool,
}

/// An instant: day plus Redmond's usual 06:59:59 UTC, unless a time is given.
fn t(day: &str, time: &str) -> i64 {
    fmt::parse_rfc3339(&format!("{day}T{time}Z")).expect("lifecycle date")
}
fn d(day: &str) -> i64 {
    t(day, "06:59:59")
}

// https://learn.microsoft.com/en-us/lifecycle/products/windows-11-home-and-pro
// (Home, Pro, Pro Education, Pro for Workstations, SE)
fn win11_home_pro(v: &str) -> Option<i64> {
    Some(match v {
        "21H2" => d("2023-10-11"),
        "22H2" => d("2024-10-09"),
        "23H2" => d("2025-11-12"),
        "24H2" => d("2026-10-14"),
        "25H2" => d("2027-10-13"),
        "26H1" => d("2028-03-15"),
        "26H2" => d("2028-10-10"),
        _ => return None,
    })
}

// https://learn.microsoft.com/en-us/lifecycle/products/windows-11-enterprise-and-education
// (Education, Enterprise, Enterprise multi-session; IoT Enterprise has the same dates:
// https://learn.microsoft.com/en-us/lifecycle/products/windows-11-iot-enterprise)
fn win11_enterprise(v: &str) -> Option<i64> {
    Some(match v {
        "21H2" => d("2024-10-09"),
        "22H2" => d("2025-10-15"),
        "23H2" => d("2026-11-11"),
        "24H2" => d("2027-10-13"),
        "25H2" => d("2028-10-11"),
        "26H1" => d("2029-03-14"),
        "26H2" => t("2029-10-09", "08:00:00"),
        _ => return None,
    })
}

// https://learn.microsoft.com/en-us/lifecycle/products/windows-10-home-and-pro
fn win10_home_pro(v: &str) -> Option<i64> {
    Some(match v {
        "22H2" => d("2025-10-15"),
        "21H2" => d("2023-06-14"),
        "21H1" => d("2022-12-14"),
        "20H2" => d("2022-05-11"),
        "2004" => d("2021-12-15"),
        "1909" => d("2021-05-12"),
        "1903" => d("2020-12-09"),
        "1809" => d("2020-11-11"),
        "1803" => d("2019-11-13"),
        "1709" => d("2019-04-10"),
        "1703" => d("2018-10-10"),
        "1607" => d("2018-04-11"),
        "1511" => d("2017-10-11"),
        "1507" => d("2017-05-10"),
        _ => return None,
    })
}

// https://learn.microsoft.com/en-us/lifecycle/products/windows-10-enterprise-and-education
// (IoT Enterprise has the same dates: https://learn.microsoft.com/en-us/lifecycle/products/windows-10-iot-enterprise)
fn win10_enterprise(v: &str) -> Option<i64> {
    Some(match v {
        "22H2" => d("2025-10-15"),
        "21H2" => d("2024-06-12"),
        "21H1" => d("2022-12-14"),
        "20H2" => d("2023-05-10"),
        "2004" => d("2021-12-15"),
        "1909" => d("2022-05-11"),
        "1903" => d("2020-12-09"),
        "1809" => d("2021-05-12"),
        "1803" => d("2021-05-12"),
        "1709" => d("2020-10-14"),
        "1703" => d("2019-10-09"),
        "1607" => d("2019-04-10"),
        "1511" => d("2017-10-11"),
        "1507" => d("2017-05-10"),
        _ => return None,
    })
}

// LTSC follows the Fixed policy and is matched by build, not by version string.
// https://learn.microsoft.com/en-us/lifecycle/products/windows-11-enterprise-ltsc-2024
// https://learn.microsoft.com/en-us/lifecycle/products/windows-11-iot-enterprise-ltsc-2024
fn win11_ltsc(build: u32, edition: EditionClass) -> Option<i64> {
    match (build, edition) {
        (26100, EditionClass::Ltsc) => Some(d("2029-10-10")),
        (26100, EditionClass::IotLtsc) => Some(d("2034-10-11")),
        _ => None,
    }
}

// https://learn.microsoft.com/en-us/lifecycle/products/windows-10-enterprise-ltsc-2021
// https://learn.microsoft.com/en-us/lifecycle/products/windows-10-iot-enterprise-ltsc-2021
// https://learn.microsoft.com/en-us/lifecycle/products/windows-10-enterprise-ltsc-2019
// https://learn.microsoft.com/en-us/lifecycle/products/windows-10-iot-enterprise-ltsc-2019
// https://learn.microsoft.com/en-us/lifecycle/products/windows-10-2016-ltsb
// https://learn.microsoft.com/en-us/lifecycle/products/windows-10-iot-enterprise-ltsb-2016
// https://learn.microsoft.com/en-us/lifecycle/products/windows-10-2015-ltsb
// https://learn.microsoft.com/en-us/lifecycle/products/windows-10-iot-enterprise-ltsb-2015
fn win10_ltsc(build: u32, edition: EditionClass) -> Option<i64> {
    match (build, edition) {
        (19044, EditionClass::Ltsc) => Some(d("2027-01-13")),
        (19044, EditionClass::IotLtsc) => Some(d("2032-01-14")),
        (17763, EditionClass::Ltsc | EditionClass::IotLtsc) => Some(d("2029-01-10")),
        (14393, EditionClass::Ltsc | EditionClass::IotLtsc) => Some(d("2026-10-14")),
        (10240, EditionClass::Ltsc | EditionClass::IotLtsc) => Some(d("2025-10-15")),
        _ => None,
    }
}

/// The newest Windows 10 22H2 build revision a PC gets without ESU: 19045.6466, KB5071959 (11 November
/// 2025, offered only to PCs not enrolled). Every later update, from 19045.6575 (KB5068781, the same
/// day) on, comes only through ESU.
/// https://support.microsoft.com/en-us/topic/windows-10-update-history-8127c2c6-6edf-4fdf-8b9f-0f7be1ef3562
pub const LAST_WIN10_UBR_WITHOUT_ESU: u32 = 6466;

// Consumer ESU for Windows 10 22H2 runs until 12 Oct 2027 (extended by Microsoft in June 2026).
fn consumer_esu_end() -> i64 {
    d("2027-10-13")
}
// Commercial ESU lasts "a maximum of three years after end of support" (Microsoft ESU page).
// Microsoft doesn't publish the final day, so this is the October 2028 Patch Tuesday: enforced,
// never printed. Recheck before each release.
fn commercial_esu_end() -> i64 {
    d("2028-10-11")
}

fn starts_with_ci(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len() && s.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

fn edition_class(edition_id: Option<&str>) -> Option<EditionClass> {
    let e = edition_id.filter(|e| !e.is_empty())?;
    if starts_with_ci(e, "IoTEnterpriseS") {
        Some(EditionClass::IotLtsc)
    } else if starts_with_ci(e, "EnterpriseS") {
        Some(EditionClass::Ltsc)
    } else if ["Core", "Professional", "CloudEdition"].iter().any(|p| starts_with_ci(e, p)) {
        Some(EditionClass::HomePro)
    } else if ["Enterprise", "Education", "ServerRdsh", "IoTEnterprise"].iter().any(|p| starts_with_ci(e, p)) {
        Some(EditionClass::Enterprise)
    } else {
        None
    }
}

/// Support end for a Windows install, or None when Fettle doesn't know this version or edition.
pub fn support_end(build: u32, display_version: Option<&str>, edition_id: Option<&str>) -> Option<SupportInfo> {
    let edition = edition_class(edition_id)?;
    let plain = |end| SupportInfo { end, esu: Esu::None, esu_end: None, show_esu_end: false, fixed: false };
    let fixed = |end| SupportInfo { fixed: true, ..plain(end) };
    let version = display_version.filter(|v| !v.is_empty());

    if build >= 22000 {
        let end = match edition {
            EditionClass::HomePro => version.and_then(win11_home_pro),
            EditionClass::Enterprise => version.and_then(win11_enterprise),
            EditionClass::Ltsc | EditionClass::IotLtsc => return win11_ltsc(build, edition).map(fixed),
        };
        return end.map(plain);
    }
    if matches!(edition, EditionClass::Ltsc | EditionClass::IotLtsc) {
        return win10_ltsc(build, edition).map(fixed);
    }
    let end = version.and_then(if edition == EditionClass::HomePro { win10_home_pro } else { win10_enterprise })?;
    // ESU only covers 22H2.
    if version != Some("22H2") {
        return Some(plain(end));
    }
    Some(if edition == EditionClass::HomePro {
        SupportInfo { end, esu: Esu::Consumer, esu_end: Some(consumer_esu_end()), show_esu_end: true, fixed: false }
    } else {
        SupportInfo { end, esu: Esu::Commercial, esu_end: Some(commercial_esu_end()), show_esu_end: false, fixed: false }
    })
}

/// The last supported day, as YYYY-MM-DD in Redmond time, for display.
pub fn last_day(end: i64) -> String {
    fmt::utc_day(end - 8 * 3600 * 1000)
}
