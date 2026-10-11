use crate::thresholds::SUMMARY_MAX_ACTIONS;
use crate::{Level, Line, MessageRef, SectionId, SectionVerdict, Summary};

/// The receipt's top line and its top actions, act_soon before keep_an_eye.
pub fn summarize(sections: &[SectionVerdict]) -> Summary {
    let lines: Vec<_> = sections.iter().flat_map(|s| &s.lines).collect();
    let has = |level: Level| lines.iter().any(|l| l.level == level);
    let storage: Vec<&Line> = sections.iter().filter(|s| s.section == SectionId::Storage).flat_map(|s| &s.lines).collect();
    // Disks that need administrator access to read: a Deep check would cover them.
    let disks_unchecked = storage.iter().any(|l| {
        l.headline.key == "storage.deep_check" || (l.id == "storage-check" && l.action.as_ref().is_some_and(|a| a.key == "check.needs_admin.action"))
    });
    // Disks whose health is unknown: a disk that doesn't share it, a disks check that didn't run,
    // or no disk found healthy at all.
    let disks_unknown = disks_unchecked
        || storage.iter().any(|l| l.headline.key == "storage.no_health" || l.id == "storage-check")
        || !storage.iter().any(|l| l.id.starts_with("disk-") && l.level == Level::Fine);
    // Only say "In fine fettle" when something was actually found to be fine, and nothing important
    // was left unchecked.
    let key = if has(Level::ActSoon) {
        "summary.attention"
    } else if has(Level::KeepAnEye) {
        "summary.some"
    } else if has(Level::Fine) && disks_unknown {
        "summary.fine_unchecked"
    } else if has(Level::Fine) {
        "summary.fine"
    } else {
        "summary.partial"
    };
    let actions_for = |level: Level| lines.iter().filter(move |l| l.level == level).filter_map(|l| l.action.clone());
    let mut actions: Vec<MessageRef> = Vec::new();
    // The Deep check comes after the warnings: it finds things out, they fix things.
    let deep_check = disks_unchecked.then(|| MessageRef::key("summary.action.deep_check"));
    for a in actions_for(Level::ActSoon).chain(actions_for(Level::KeepAnEye)).chain(deep_check) {
        if !actions.iter().any(|b| b.key == a.key) {
            actions.push(a);
        }
    }
    actions.truncate(SUMMARY_MAX_ACTIONS);
    Summary { key: key.into(), actions }
}
