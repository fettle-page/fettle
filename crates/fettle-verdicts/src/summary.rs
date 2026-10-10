use crate::thresholds::SUMMARY_MAX_ACTIONS;
use crate::{Level, MessageRef, SectionVerdict, Summary};

/// The receipt's top line and its top actions, act_soon before keep_an_eye.
pub fn summarize(sections: &[SectionVerdict]) -> Summary {
    let lines: Vec<_> = sections.iter().flat_map(|s| &s.lines).collect();
    let has = |level: Level| lines.iter().any(|l| l.level == level);
    // Disks that need administrator access to read: a Deep check would cover them.
    let disks_unchecked = lines.iter().any(|l| l.headline.key == "storage.deep_check");
    // Only say "In fine fettle" when something was actually found to be fine, and nothing important
    // was left unchecked.
    let key = if has(Level::ActSoon) {
        "summary.attention"
    } else if has(Level::KeepAnEye) {
        "summary.some"
    } else if has(Level::Fine) && disks_unchecked {
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
