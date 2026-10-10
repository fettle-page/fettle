use crate::thresholds::SUMMARY_MAX_ACTIONS;
use crate::{Level, MessageRef, SectionVerdict, Summary};

/// The receipt's top line and its top actions, act_soon before keep_an_eye.
pub fn summarize(sections: &[SectionVerdict]) -> Summary {
    let lines: Vec<_> = sections.iter().flat_map(|s| &s.lines).collect();
    let has = |level: Level| lines.iter().any(|l| l.level == level);
    // Only say "In fine fettle" when something was actually found to be fine.
    let key = if has(Level::ActSoon) {
        "summary.attention"
    } else if has(Level::KeepAnEye) {
        "summary.some"
    } else if has(Level::Fine) {
        "summary.fine"
    } else {
        "summary.partial"
    };
    let actions_for = |level: Level| lines.iter().filter(move |l| l.level == level).filter_map(|l| l.action.clone());
    let mut actions: Vec<MessageRef> = Vec::new();
    for a in actions_for(Level::ActSoon).chain(actions_for(Level::KeepAnEye)) {
        if !actions.iter().any(|b| b.key == a.key) {
            actions.push(a);
        }
    }
    actions.truncate(SUMMARY_MAX_ACTIONS);
    Summary { key: key.into(), actions }
}
