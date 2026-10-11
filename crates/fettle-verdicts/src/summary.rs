use crate::thresholds::SUMMARY_MAX_ACTIONS;
use crate::{Level, Line, Locale, MessageRef, SectionId, SectionVerdict, Summary, render};

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
    // Anything else left unknown: a section that couldn't be read, or a Windows Fettle can't judge.
    // The TPM line on Windows 10 is about readiness for Windows 11, not this PC's health.
    let windows_fine = lines.iter().any(|l| l.id == "windows-support" && l.level == Level::Fine);
    let others_unknown = !windows_fine
        || sections.iter().filter(|s| s.section != SectionId::Storage).flat_map(|s| &s.lines).any(|l| l.level == Level::CouldntCheck && l.id != "windows-tpm");
    // Only say "In fine fettle" when something was actually found to be fine, and nothing important
    // was left unchecked.
    let key = if has(Level::ActSoon) {
        "summary.attention"
    } else if has(Level::KeepAnEye) {
        "summary.some"
    } else if has(Level::Fine) && others_unknown {
        "summary.fine_partial"
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
        // The same step twice, even under two keys, reads as a mistake: compare the words too.
        let same = |b: &MessageRef| b.key == a.key || [Locale::En, Locale::It].iter().all(|&l| render(b, l) == render(&a, l));
        if !actions.iter().any(same) {
            actions.push(a);
        }
    }
    actions.truncate(SUMMARY_MAX_ACTIONS);
    Summary { key: key.into(), actions }
}
