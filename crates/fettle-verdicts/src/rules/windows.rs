use super::common::{Action, couldnt_check, line, section_problem};
use crate::lifecycle::{Esu, LAST_WIN10_UBR_WITHOUT_ESU, last_day, support_end, win10_updates_missed_after};
use crate::thresholds::{SUPPORT_END_KEEP_AN_EYE_DAYS, UPTIME_NUDGE_DAYS};
use crate::{Level, Line, MessageRef, Param, SectionId, SectionVerdict};
use fettle_facts::{EsuState, Section, Windows};

/// `cpu_name` is the processor as Windows names it (from the system facts), for Windows 11 readiness.
pub fn windows_rules(s: Option<&Section<Windows>>, now_ms: i64, elevated: bool, cpu_name: Option<&str>) -> SectionVerdict {
    let problem = section_problem(SectionId::Windows, s.map(|s| &s.check));
    let w = s.and_then(|s| s.data.as_ref()).filter(|_| problem.is_none());
    let Some((w, build)) = w.and_then(|w| w.build.map(|b| (w, b))) else {
        return SectionVerdict { section: SectionId::Windows, lines: couldnt_check(SectionId::Windows, problem) };
    };

    let mut lines = vec![support_line(w, build, now_ms)];

    let days = w.uptime_seconds.unwrap_or(0) / 86400;
    if days >= UPTIME_NUDGE_DAYS {
        lines.push(line("windows-uptime", Level::Info, "windows.restart_nudge", &[("days", Param::Int(days as i64))], Action::Default));
    }

    if build < 22000 {
        lines.extend(readiness_lines(w, elevated, cpu_name));
    } else if w.secure_boot == Some(false) {
        // Windows 11 runs with Secure Boot off; say why turning it on helps, without calling it a fault.
        lines.push(line("windows-secure-boot", Level::Info, "windows.secure_boot_off_w11", &[], Action::Default));
    }
    SectionVerdict { section: SectionId::Windows, lines }
}

fn support_line(w: &Windows, build: u32, now: i64) -> Line {
    let product: Param = w.product.clone().unwrap_or_else(|| "Windows".into()).into();
    let product = || ("product", product.clone());
    // Never guess "act soon" for a version we don't know.
    let Some(info) = support_end(build, w.display_version.as_deref(), w.edition_id.as_deref()) else {
        return line("windows-support", Level::Info, "windows.unknown_version", &[product()], Action::Default);
    };

    if now <= info.end {
        let until = || ("until", Param::from(last_day(info.end)));
        // Already enrolled in ESU: updates carry on past the end, nothing to do.
        let enrolled = w.esu == Some(EsuState::Enrolled);
        if info.end - now > SUPPORT_END_KEEP_AN_EYE_DAYS * 86_400_000 || enrolled {
            return line("windows-support", Level::Fine, "windows.supported", &[product(), until()], Action::Default);
        }
        // Ending soon: the same way out as once it has ended, while there's still time.
        // LTSC moves to a newer LTSC release, not to a feature update.
        let key = if build >= 22000 && !info.fixed {
            "windows.ending_feature_update"
        } else if info.esu != Esu::None {
            "windows.ending_esu"
        } else {
            "windows.ending"
        };
        return line("windows-support", Level::KeepAnEye, key, &[product(), until()], Action::Default);
    }
    // Out of support, and ESU can't help: not 22H2, LTSC, or ESU itself has ended.
    let esu_over = info.esu_end.is_some_and(|end| now > end);
    if info.esu == Esu::None || esu_over {
        // An old Windows 11 release just needs the latest (free) feature update.
        let key = if build >= 22000 && !info.fixed { "windows.unsupported_feature_update" } else { "windows.unsupported" };
        return line("windows-support", Level::ActSoon, key, &[product()], Action::Default);
    }

    // Still on the last build without ESU long after it: no security fixes since, enrolled or not.
    let stale = build == 19045 && w.ubr.is_some_and(|u| u <= LAST_WIN10_UBR_WITHOUT_ESU) && now > win10_updates_missed_after();
    let state = esu_state(w, build);
    if stale {
        // Enrolled already: the updates only need installing.
        let action = if state == Some(EsuState::Enrolled) { "windows.esu_stale_enrolled.action" } else { "windows.esu_stale.action" };
        return line("windows-support", Level::ActSoon, "windows.esu_stale", &[product()], Action::Key(action));
    }
    match state {
        Some(EsuState::Enrolled) => {
            // ESU's own end gets the same warning window. Commercial ESU's paid-for years aren't
            // known, but its last possible day is, so near it the date is exact.
            let ending = info.esu_end.filter(|end| end - now <= SUPPORT_END_KEEP_AN_EYE_DAYS * 86_400_000);
            match (ending, info.esu_end.filter(|_| info.show_esu_end)) {
                (Some(end), _) => line("windows-support", Level::KeepAnEye, "windows.esu_ending", &[product(), ("until", last_day(end).into())], Action::Default),
                (None, Some(end)) => line("windows-support", Level::Fine, "windows.esu_enrolled_until", &[product(), ("until", last_day(end).into())], Action::Default),
                (None, None) => line("windows-support", Level::Fine, "windows.esu_enrolled", &[product()], Action::Default),
            }
        }
        Some(EsuState::NotEnrolled) => line("windows-support", Level::ActSoon, "windows.unsupported_esu", &[product()], Action::Default),
        _ => line("windows-support", Level::KeepAnEye, "windows.esu_unknown", &[product()], Action::Default),
    }
}

/// Whether ESU is delivering updates. An explicit answer wins; otherwise the build shows it, because
/// every Windows 10 22H2 update after the last one without ESU comes only through ESU. (A Windows 10
/// laptop reported no ESU licence while installing monthly security updates.) When an update was
/// installed proves nothing: PCs install old updates late.
fn esu_state(w: &Windows, build: u32) -> Option<EsuState> {
    if matches!(w.esu, Some(EsuState::Enrolled | EsuState::NotEnrolled)) {
        return w.esu;
    }
    match w.ubr {
        Some(ubr) if build == 19045 && ubr > LAST_WIN10_UBR_WITHOUT_ESU => Some(EsuState::Enrolled),
        _ => w.esu,
    }
}

/// Windows 11 readiness, shown only on Windows 10. Informational: these aren't faults.
fn readiness_lines(w: &Windows, elevated: bool, cpu_name: Option<&str>) -> Vec<Line> {
    let key = match super::cpu::win11_cpu(cpu_name) {
        super::cpu::Win11Cpu::Supported => "windows.cpu_ready",
        super::cpu::Win11Cpu::Unsupported => "windows.cpu_unsupported",
        super::cpu::Win11Cpu::Unknown => "windows.cpu_unknown",
    };
    let mut lines = vec![line("windows-cpu", Level::Info, key, &[], Action::Default)];
    // A TPM that is off or not ready in the firmware answers with no version: that says nothing
    // about which version it is.
    let version = w.tpm.as_ref().and_then(|t| t.spec_version.as_deref()).map(str::trim).filter(|v| !v.is_empty());
    match &w.tpm {
        Some(tpm) if tpm.present == Some(false) => lines.push(line("windows-tpm", Level::Info, "windows.no_tpm", &[], Action::Default)),
        Some(_) if version.is_some() => {
            let v2 = version.is_some_and(|v| v.starts_with("2.0"));
            lines.push(line("windows-tpm", Level::Info, if v2 { "windows.tpm_ready" } else { "windows.tpm_old" }, &[], Action::Default));
        }
        _ => {
            let mut l = line("windows-tpm", if elevated { Level::Info } else { Level::CouldntCheck }, "windows.tpm_unknown", &[], Action::Default);
            if !elevated {
                l.action = Some(MessageRef::key("check.needs_admin.action"));
            }
            lines.push(l);
        }
    }
    if let Some(on) = w.secure_boot {
        lines.push(line("windows-secure-boot", Level::Info, if on { "windows.secure_boot_on" } else { "windows.secure_boot_off" }, &[], Action::Default));
    }
    lines
}
