use crate::{Level, Line, MessageRef, Param, SectionId};
use fettle_facts::{Check, CheckStatus};

/// Which action a line gets.
pub enum Action<'a> {
    /// Warnings (keep_an_eye, act_soon) get `<key>.action`; other levels none.
    Default,
    /// This key (singular `_one` lines share their plural's action).
    Key(&'a str),
}

/// A verdict line.
pub fn line(id: impl Into<String>, level: Level, key: &str, params: &[(&str, Param)], action: Action) -> Line {
    let headline = if params.is_empty() { MessageRef::key(key) } else { MessageRef::with(key, params) };
    let warns = matches!(level, Level::KeepAnEye | Level::ActSoon);
    let action = match action {
        Action::Default if warns => Some(MessageRef::key(format!("{key}.action"))),
        Action::Default => None,
        Action::Key(k) => Some(MessageRef::key(k)),
    };
    Line { id: id.into(), level, headline, detail: None, action }
}

/// `base` or `base_one`, for the copy's singular forms.
pub fn plural(base: &str, count: u64) -> String {
    if count == 1 { format!("{base}_one") } else { base.to_string() }
}

fn status_key(s: CheckStatus) -> &'static str {
    match s {
        CheckStatus::Ok => "ok",
        CheckStatus::NeedsAdmin => "needs_admin",
        CheckStatus::NotPresent => "not_present",
        CheckStatus::Failed => "failed",
    }
}

/// For a section that's missing or wasn't read: one "Couldn't check" line saying why.
pub fn section_problem(section: SectionId, check: Option<&Check>) -> Option<Line> {
    let status = check.map(|c| c.status).unwrap_or(CheckStatus::Failed);
    if status == CheckStatus::Ok {
        return None;
    }
    let name = section.key();
    let mut l = line(format!("{name}-check"), Level::CouldntCheck, &format!("{name}.couldnt_check"), &[], Action::Default);
    l.detail = Some(MessageRef::key(format!("check.{}", status_key(status))));
    if status == CheckStatus::NeedsAdmin {
        l.action = Some(MessageRef::key("check.needs_admin.action"));
    }
    Some(l)
}

/// The section's only line when it has no data: its problem, or a plain "couldn't check".
pub fn couldnt_check(section: SectionId, problem: Option<Line>) -> Vec<Line> {
    let name = section.key();
    vec![problem.unwrap_or_else(|| line(format!("{name}-check"), Level::CouldntCheck, &format!("{name}.couldnt_check"), &[], Action::Default))]
}

/// A count as a message param (counts beyond i64 can't happen on real hardware; they saturate).
pub fn int(n: u64) -> Param {
    Param::Int(i64::try_from(n).unwrap_or(i64::MAX))
}
