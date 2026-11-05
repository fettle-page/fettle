//! Puts the state on the window: which views show, the receipt (built by fettle-receipt in the
//! viewer's language and time zone), the panel's texts and the status bar.

use crate::copy::t;
use crate::state::{Deep, MainView, Mat, Panel, PrivateCard, PrivateKind, ShareStatus, State, ThemeChoice};
use crate::{AppWindow, CheckRow, Copy, Group, Hint, Line, LinkRow, ReceiptData, Row, Section, SlipData, SlipGroup, SlipRow, TabRef, TabRow, Theme, sections};
use fettle_facts::Report;
use fettle_receipt::{DetailRow, ReceiptModel, Zone};
use fettle_verdicts::{Locale, Param, Receipt, fmt};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::time::Duration;

/// What the window last got for each list. Setting a new list rebuilds everything shown from it
/// (an open fold closes, a focused button loses the keyboard), so lists are set only on change.
#[derive(Default)]
pub struct Cache {
    receipt: String,
    numbers: String,
    blank: String,
    links: String,
    private: String,
    preview: String,
    checks: String,
    slip: String,
    notes: String,
    technician: String,
}

/// The print preview's pages, drawn when it opens and dropped when it closes.
pub struct Preview {
    pub pages: Vec<slint::Image>,
    pub paper: crate::print::layout::PageSize,
    /// Whether Windows can print here (otherwise the preview saves a PDF).
    pub printing: bool,
    /// Which drawing this is, so the window only gets new pages when they change.
    pub generation: u64,
}

/// Runs `set` only when `key` differs from what was set last time.
fn changed(slot: &mut String, key: String, set: impl FnOnce()) {
    if *slot != key {
        set();
        *slot = key;
    }
}

pub struct Ctx<'a> {
    pub system_lang: Locale,
    pub zone: &'a dyn Fn(i64) -> i32,
    pub now_ms: i64,
    /// The privacy switch's details, while it's on and they've been read.
    pub private: Option<&'a PrivateCard>,
    /// The print preview's pages, while it's open.
    pub preview: Option<&'a Preview>,
}

fn model<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(items))
}

fn strings(items: Vec<String>) -> ModelRc<SharedString> {
    model(items.into_iter().map(SharedString::from).collect())
}

/// The verdicts and the receipt for a report, with its time where the viewer is.
fn receipt_of(report: &Report, lang: Locale, ctx: &Ctx) -> Option<(Receipt, ReceiptModel)> {
    let at = fmt::parse_rfc3339(&report.collected_at)?;
    fettle_receipt::receipt_for(report, lang, Zone::Offset((ctx.zone)(at))).ok()
}

/// Two checks before and after, each stamped with the offset of its own day, as its receipt is.
pub fn comparison(a: &Report, b: &Report, lang: Locale, zone: &dyn Fn(i64) -> i32) -> Option<fettle_receipt::Comparison> {
    fettle_receipt::compare(a, b, lang, |at| Zone::Offset(zone(at))).ok()
}

/// The comparison as the slip on the mat draws it: better and worse each with its word.
fn slip_of(c: &fettle_receipt::Comparison) -> SlipData {
    use fettle_receipt::Change;
    let groups = c.groups.iter().map(|g| SlipGroup {
        title: g.title.clone().into(),
        rows: model(
            g.rows
                .iter()
                .map(|r| SlipRow {
                    label: r.label.clone().into(),
                    before: r.before.clone().into(),
                    after: r.after.clone().into(),
                    change: level_key(r.change).into(),
                    word: match r.change {
                        Change::Better => c.labels.better.clone(),
                        Change::Worse => c.labels.worse.clone(),
                        Change::Neutral => String::new(),
                    }
                    .into(),
                })
                .collect(),
        ),
    });
    SlipData {
        title: c.labels.title.clone().into(),
        before_label: c.labels.before.clone().into(),
        after_label: c.labels.after.clone().into(),
        before: c.before.clone().into(),
        after: c.after.clone().into(),
        summary_before: c.summary_before.clone().into(),
        summary_after: c.summary_after.clone().into(),
        different_pc: c.different_pc.clone().unwrap_or_default().into(),
        nothing_changed: c.nothing_changed.clone().unwrap_or_default().into(),
        groups: model(groups.collect()),
    }
}

/// My checks' rows: when, Quick or Deep, the summary, and a name for each control.
fn check_rows(s: &State, lang: Locale, ctx: &Ctx) -> Vec<CheckRow> {
    s.checks
        .iter()
        .filter_map(|c| {
            let (_, m) = receipt_of(&c.report, lang, ctx)?;
            let when = m.meta.when.clone();
            let label = |key: &str| SharedString::from(t(key, lang, &[("when", when.clone().into())]));
            Some(CheckRow {
                id: c.id().into(),
                kind: t(if c.report.elevated { "checks.deep" } else { "checks.quick" }, lang, &[]).into(),
                summary: m.summary.clone().into(),
                open_label: label("checks.open_label"),
                delete_label: label("checks.delete_label"),
                tick_label: label("checks.tick_label"),
                when: when.into(),
            })
        })
        .collect()
}

/// Whole calendar days between the check and now, where the viewer is. Never negative.
fn days_ago(report: &Report, ctx: &Ctx) -> i64 {
    let Some(at) = fmt::parse_rfc3339(&report.collected_at) else { return 0 };
    let day = |ms: i64| (ms + i64::from((ctx.zone)(ms)) * 60_000).div_euclid(86_400_000);
    (day(ctx.now_ms) - day(at)).max(0)
}

fn rows(rows: &[DetailRow]) -> ModelRc<Row> {
    model(rows.iter().map(|r| Row { label: r.label.clone().into(), value: r.value.clone().into() }).collect())
}

fn level_key<T: serde::Serialize>(level: T) -> String {
    serde_json::to_value(level).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

fn sections_of(m: &ReceiptModel, lang: Locale) -> Vec<Section> {
    let mut order = 0;
    m.sections
        .iter()
        .map(|s| {
            let id = level_key(s.id);
            Section {
                // A tab is narrower than a receipt heading: "Spazio di archiviazione" becomes "Dischi".
                tab: if id == "storage" { t("numbers.storage", lang, &[]) } else { s.title.clone() }.into(),
                id: id.into(),
                title: s.title.clone().into(),
                lines: model(
                    s.lines
                        .iter()
                        .map(|l| {
                            order += 1;
                            Line {
                                level: level_key(l.level).into(),
                                tag: l.tag.clone().into(),
                                headline: l.headline.clone().into(),
                                detail: l.detail.clone().unwrap_or_default().into(),
                                action: l.action.clone().unwrap_or_default().into(),
                                order: order - 1,
                            }
                        })
                        .collect(),
                ),
                details: model(s.details.iter().map(|g| Group { title: g.title.clone().unwrap_or_default().into(), rows: rows(&g.rows) }).collect()),
            }
        })
        .collect()
}

pub fn receipt_data(m: &ReceiptModel, lang: Locale) -> ReceiptData {
    ReceiptData {
        tone: m.tone.clone().into(),
        when: m.meta.when.clone().into(),
        summary: m.summary.clone().into(),
        actions: strings(m.actions.clone()),
        machine: rows(&m.machine),
        sections: model(sections_of(m, lang)),
        scan: m.meta.scan.clone().into(),
        footer: m.meta.footer.clone().into(),
        stamp: m.labels.stamp.clone().into(),
        next_step: m.labels.next_step.clone().into(),
        details_label: m.labels.details.clone().into(),
    }
}

/// Whether a Deep check would show more: something needed administrator access and didn't get it.
pub fn wants_deep_check(receipt: &Receipt, elevated: bool) -> bool {
    const DEEP_ACTIONS: [&str; 2] = ["check.needs_admin.action", "storage.deep_check.action"];
    !elevated && receipt.sections.iter().any(|s| s.lines.iter().any(|l| l.action.as_ref().is_some_and(|a| DEEP_ACTIONS.contains(&a.key.as_str()))))
}

fn private_rows(card: &PrivateCard, lang: Locale) -> Vec<Row> {
    let mut rows = Vec::new();
    if let Some(name) = &card.pc_name {
        rows.push(Row { label: t("private.pc", lang, &[]).into(), value: name.clone().into() });
    }
    for (kind, serial) in &card.items {
        let label = match kind {
            PrivateKind::System => t("private.system", lang, &[]),
            PrivateKind::Board => t("private.board", lang, &[]),
            PrivateKind::Disk(model) => t("private.disk", lang, &[("label", model.clone().unwrap_or_else(|| "?".into()).into())]),
        };
        rows.push(Row { label: label.into(), value: serial.clone().into() });
    }
    rows
}

/// "Windows 11 Pro 24H2", for the status bar.
fn windows_of(report: &Report) -> Option<String> {
    let w = report.facts.windows.as_ref()?.data.as_ref()?;
    let product = w.product.clone().filter(|p| !p.is_empty())?;
    Some([Some(product), w.display_version.clone().filter(|v| !v.is_empty())].into_iter().flatten().collect::<Vec<_>>().join(" "))
}

/// "In fine fettle." → "in fine fettle.", to follow "Last time: ".
fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_lowercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

fn hint(keys: &str, label: String) -> Hint {
    Hint { keys: keys.into(), label: label.into() }
}

/// An Italian long date ("8 ottobre 2026") after the article its day needs: "l'8", "l'11", "il 1°"
/// and "il 17". Templates can't choose it, so the app fills the whole phrase.
fn it_dated(date: &str, plain: &str, elided: &str) -> String {
    let day: String = date.chars().take_while(char::is_ascii_digit).collect();
    match day.as_str() {
        "8" | "11" => format!("{elided}{date}"),
        "1" => format!("{plain} 1°{}", &date[1..]),
        _ => format!("{plain} {date}"),
    }
}

/// "il 17 ottobre", "l'8 ottobre", "il 1° ottobre".
fn it_il(date: &str) -> String {
    it_dated(date, "il", "l'")
}

/// "al 17 ottobre", "all'8 ottobre", "al 1° ottobre".
fn it_al(date: &str) -> String {
    it_dated(date, "al", "all'")
}

/// A date for a template: plain as `{date}`, and with its Italian article as `{il_date}` and
/// `{al_date}` (English templates just use `{date}`).
fn date_params(date: String) -> Vec<(&'static str, Param)> {
    vec![("il_date", it_il(&date).into()), ("al_date", it_al(&date).into()), ("date", date.into())]
}

/// Seconds with one decimal in the language ("0.7", "0,7").
fn seconds(s: f64, lang: Locale) -> String {
    let text = format!("{s:.1}");
    if lang == Locale::It { text.replace('.', ",") } else { text }
}

pub fn apply(w: &AppWindow, s: &State, ctx: &Ctx, cache: &mut Cache) {
    let lang = s.lang(ctx.system_lang);
    w.global::<Copy>().set_lang(lang.code().into());
    w.global::<Theme>().set_choice(
        match s.settings.theme {
            ThemeChoice::Auto => "system",
            ThemeChoice::Light => "light",
            ThemeChoice::Dark => "dark",
            ThemeChoice::Oled => "oled",
        }
        .into(),
    );
    w.set_lang_choice(s.settings.lang.map(|l| l.code()).unwrap_or("auto").into());
    w.set_theme_choice(w.global::<Theme>().get_choice());
    w.set_remember(s.settings.remember);
    w.set_show_private(s.settings.show_private);
    w.set_updates(s.settings.updates);
    w.set_update_every(
        match s.settings.update_every {
            crate::state::UpdateEvery::Hourly => "hourly",
            crate::state::UpdateEvery::Daily => "daily",
            crate::state::UpdateEvery::Weekly => "weekly",
        }
        .into(),
    );
    w.set_version(t("about.version", lang, &[("version", env!("CARGO_PKG_VERSION").into())]).into());

    // The privacy card: only while the switch is on and the details have been read.
    let card = ctx.private.filter(|_| s.private_shown && s.settings.show_private);
    w.set_private_shown(card.is_some());
    let rows = card.map(|c| private_rows(c, lang)).unwrap_or_default();
    changed(&mut cache.private, rows.iter().map(|r| format!("{}={}", r.label, r.value)).collect::<Vec<_>>().join("|"), || w.set_private_rows(model(rows.clone())));

    changed(&mut cache.blank, format!("{}|{}", lang.code(), s.done_sources.join(",")), || {
        let (machine, blank) = sections::blank(&s.done_sources, lang);
        w.set_blank_machine(machine);
        w.set_blank_sections(blank);
        let (progress, count) = sections::progress(&s.done_sources, lang);
        w.set_progress(progress);
        w.set_progress_count(count.into());
    });

    let main = s.main_view();
    w.set_view(
        match s.panel {
            Panel::Settings => "settings",
            Panel::Share => "share",
            Panel::Links => "links",
            Panel::Preview => "preview",
            Panel::Checks => "checks",
            Panel::Compare => "compare",
            Panel::Main => match main {
                MainView::Home => "home",
                MainView::Checking => "checking",
                MainView::Result => "result",
                MainView::Past => "past",
            },
        }
        .into(),
    );
    w.set_mat(
        match s.mat() {
            Mat::Blank => "blank",
            Mat::Live => "live",
            Mat::Last => "last",
            Mat::Past => "past",
        }
        .into(),
    );
    w.set_alert(s.problem.map(|p| SharedString::from(t(p.key(), lang, &[]))).unwrap_or_default());

    let current = s.current.as_ref().and_then(|c| receipt_of(&c.report, lang, ctx).map(|r| (c, r)));
    let last = s.last_check().and_then(|r| receipt_of(r, lang, ctx).map(|m| (r, m)));
    let past = s.opened_check().and_then(|c| receipt_of(&c.report, lang, ctx));

    // The receipt on the mat: this check's, the last one, or a past one opened from My checks.
    let on_mat = match (s.mat(), &current, &last, &past) {
        (Mat::Live, Some((_, (_, m))), _, _) | (Mat::Last, _, Some((_, (_, m))), _) | (Mat::Past, _, _, Some((_, m))) => Some(m),
        _ => None,
    };
    if let Some(m) = on_mat {
        changed(&mut cache.receipt, format!("{:?}|{}|{}|{}", s.mat(), lang.code(), m.meta.collected_at, m.meta.when), || w.set_receipt(receipt_data(m, lang)));
    }

    // Coming back: what the last check found and how long ago.
    if let Some((report, (_, m))) = &last {
        let days = days_ago(report, ctx);
        let age = |today: &str, yesterday: &str, n: &str| match days {
            0 => t(today, lang, &[]),
            1 => t(yesterday, lang, &[]),
            d => t(n, lang, &[("days", Param::Int(d))]),
        };
        w.set_back_title(t("home.back_title", lang, &[("verdict", lower_first(&m.summary).into())]).into());
        w.set_back_body(age("home.back_body_today", "home.back_body_yesterday", "home.back_body").into());
        w.set_last_top(t("stamp.last", lang, &[]).into());
        w.set_last_bottom(age("stamp.today", "stamp.yesterday", "stamp.days").into());
    } else {
        w.set_back_title(SharedString::new());
    }
    // A saved check opened from My checks: its own label, with its date and time.
    if let (Mat::Past, Some((_, m))) = (s.mat(), &past) {
        w.set_last_top(t("stamp.saved", lang, &[]).into());
        w.set_last_bottom(m.meta.when.clone().into());
    }

    // My checks, and before and after.
    changed(&mut cache.checks, format!("{}|{}", lang.code(), s.checks.iter().map(|c| c.id()).collect::<Vec<_>>().join(",")), || w.set_checks(model(check_rows(s, lang, ctx))));
    w.set_checks_label(t("title.checks", lang, &[("count", Param::Int(s.checks.len() as i64))]).into());
    w.set_can_compare_previous(s.previous_check().is_some());
    let compared = s.compare.as_ref().filter(|_| s.compared().is_some());
    changed(&mut cache.slip, compared.map(|(a, b)| format!("{}|{a}|{b}", lang.code())).unwrap_or_default(), || {
        let Some((a, b)) = s.compared() else { return };
        w.set_slip(match comparison(a, b, lang, ctx.zone) {
            Some(c) => slip_of(&c),
            None => SlipData { title: t("compare.view_title", lang, &[]).into(), nothing_changed: t("compare.failed", lang, &[]).into(), ..SlipData::default() },
        });
    });

    // What the printout's fields hold: set only when the receipt shown or what's saved for it
    // changes, so a field being typed in is never overwritten.
    let shown_id = s.opened_check().map(|c| c.id()).or(s.current.as_ref().map(|c| c.report.collected_at.as_str())).unwrap_or("");
    let (client, notes) = s.shown_notes();
    changed(&mut cache.notes, format!("{shown_id}\u{1}{client}\u{1}{notes}"), || {
        w.set_client(client.into());
        w.set_notes(notes.into());
    });
    let tech = &s.settings.technician;
    changed(&mut cache.technician, format!("{}\u{1}{}\u{1}{}", tech.business, tech.name, tech.contact), || {
        w.set_tech_business(tech.business.clone().into());
        w.set_tech_name(tech.name.clone().into());
        w.set_tech_contact(tech.contact.clone().into());
    });
    w.set_has_logo(s.has_logo);
    w.set_logo_problem(s.logo_problem.map(|k| t(k, lang, &[])).unwrap_or_default().into());

    // After a check: the answer, the Deep check when it would show more, the numbers.
    if let Some((checked, (receipt, m))) = &current {
        // The first step; with none, "nothing to do", unless Fettle could hardly check anything, when
        // that would read as if all were well: then the receipt's own top line.
        let none = || if receipt.summary.key == "summary.partial" { m.summary.clone() } else { t("result.nothing", lang, &[]) };
        w.set_answer(m.actions.first().cloned().unwrap_or_else(none).into());
        w.set_deep(wants_deep_check(receipt, checked.report.elevated));
        changed(&mut cache.numbers, format!("{}|{}|{}", lang.code(), m.meta.collected_at, m.meta.when), || {
            let numbers: Vec<Section> = sections_of(m, lang).into_iter().filter(|sec| slint::Model::row_count(&sec.details) > 0).collect();
            let tabs: Vec<SharedString> = numbers.iter().map(|sec| sec.tab.clone()).collect();
            w.set_number_rows(model(tab_rows(&tabs).into_iter().map(|row| TabRow { tabs: model(row) }).collect()));
            w.set_numbers(model(numbers));
        });
    }
    w.set_deep_asking(s.deep == Deep::Asking);
    w.set_deep_problem(match &s.deep {
        Deep::Problem(key) => t(key, lang, &[]).into(),
        _ => SharedString::new(),
    });
    w.set_dock_status(s.export.map(|k| t(k, lang, &[])).unwrap_or_default().into());

    // Share and the links this PC can still delete.
    let long_local_date = |iso: &str| {
        let at = fmt::parse_rfc3339(iso).unwrap_or(0);
        fmt::date_long(&fmt::utc_day(at + i64::from((ctx.zone)(at)) * 60_000), lang)
    };
    w.set_share_status(
        match &s.share {
            ShareStatus::Editing => "editing",
            ShareStatus::Sending => "sending",
            ShareStatus::Done(_) => "done",
            ShareStatus::Failed(..) => "failed",
        }
        .into(),
    );
    w.set_share_problem(match &s.share {
        ShareStatus::Failed(key, minutes) => t(key, lang, &minutes.map(|m| vec![("minutes", Param::Int(m))]).unwrap_or_default()).into(),
        _ => SharedString::new(),
    });
    if let ShareStatus::Done(shared) = &s.share {
        w.set_share_link(shared.url.clone().into());
        if let Some(qr) = crate::share::qr_code(&shared.url) {
            w.set_share_qr(qr.path.into());
            w.set_share_qr_size(qr.size as i32);
        }
        w.set_share_locked_note(if shared.locked { t("share.locked_note", lang, &[]) } else { String::new() }.into());
        w.set_share_delete_link(shared.delete_url.clone().into());
        w.set_share_expires(t("share.expires", lang, &date_params(long_local_date(&shared.expires_at))).into());
    }
    if let Some(shown) = s.shown_report() {
        // Exactly what would leave this PC: the redacted report and the engine version.
        let outgoing = Report { engine_version: Some(fettle_verdicts::ENGINE_VERSION.to_string()), ..shown.clone() };
        w.set_share_json(serde_json::to_string_pretty(&outgoing).unwrap_or_default().into());
    }
    w.set_share_note(s.share_note.map(|k| t(k, lang, &[])).unwrap_or_default().into());
    let links: Vec<LinkRow> = s.links.iter().map(|l| LinkRow { id: l.id.clone().into(), url: l.url.clone().into(), until: t("links.until", lang, &date_params(long_local_date(&l.expires_at))).into() }).collect();
    changed(&mut cache.links, links.iter().map(|l| format!("{}={}", l.id, l.until)).collect::<Vec<_>>().join("|"), || w.set_links(model(links.clone())));
    w.set_links_label(t("title.links", lang, &[("count", Param::Int(s.links.len() as i64))]).into());
    w.set_links_status(s.links_status.map(|k| t(k, lang, &[("count", Param::Int(s.links_gone as i64))])).unwrap_or_default().into());
    w.set_links_busy(s.deleting.is_some() || s.checking_links);
    w.set_links_checking(s.checking_links);

    // The status bar: what happened, which Windows, and the shortcuts that work right now.
    let local_date = |report: &Report| {
        let at = fmt::parse_rfc3339(&report.collected_at).unwrap_or(0);
        fmt::date_long(&fmt::utc_day(at + i64::from((ctx.zone)(at)) * 60_000), lang)
    };
    let (items, hints) = match (main, &current, &last) {
        (MainView::Checking, _, _) => (vec![t("status.checking", lang, &[])], vec![]),
        (MainView::Past, _, _) if s.opened_check().is_some() => {
            let report = &s.opened_check().expect("a saved check").report;
            ([Some(t("status.saved", lang, &[("date", local_date(report).into())])), windows_of(report)].into_iter().flatten().collect(), vec![hint("Ctrl P", t("export.print", lang, &[]))])
        }
        (MainView::Result, Some((c, _)), _) => {
            let secs = seconds(c.seconds, lang);
            let key = if c.report.elevated { "status.deep_time" } else { "status.quick_time" };
            (
                [Some(t(key, lang, &[("s", secs.into())])), windows_of(&c.report)].into_iter().flatten().collect(),
                vec![hint("Ctrl R", t("scan.again", lang, &[])), hint("Ctrl P", t("export.print", lang, &[]))],
            )
        }
        (_, _, Some((report, _))) => ([Some(t("status.last", lang, &[("date", local_date(report).into())])), windows_of(report)].into_iter().flatten().collect(), vec![hint("Enter", t("scan.again", lang, &[]))]),
        _ => (vec![t("status.ready", lang, &[])], vec![hint("Enter", t("home.button", lang, &[]))]),
    };
    w.set_status_items(strings(items));

    // The print preview: its pages, how many and on what paper, and whether it prints or saves.
    let preview = ctx.preview.filter(|_| s.panel == Panel::Preview);
    changed(&mut cache.preview, preview.map(|p| p.generation).unwrap_or(0).to_string(), || {
        w.set_preview_pages(model(preview.map(|p| p.pages.clone()).unwrap_or_default()));
    });
    w.set_preview_label(
        preview
            .map(|p| {
                let paper = match p.paper {
                    crate::print::layout::PageSize::A4 => "A4",
                    crate::print::layout::PageSize::Letter => "Letter",
                    crate::print::layout::PageSize::Custom(..) => "",
                };
                let key = if p.pages.len() == 1 { "preview.page" } else { "preview.pages" };
                t(key, lang, &[("count", Param::Int(p.pages.len() as i64)), ("paper", paper.into())])
            })
            .unwrap_or_default()
            .into(),
    );
    w.set_preview_printing(preview.is_none_or(|p| p.printing));
    // A newer version, with its Download link.
    let note = s.update_note();
    w.set_update_text(note.map(|l| t("update.available", lang, &[("version", l.version.clone().into())])).unwrap_or_default().into());
    w.set_update_label(note.map(|l| t("update.get_label", lang, &[("version", l.version.clone().into())])).unwrap_or_default().into());
    // Shortcut hints only where they work: the main view (Share, Links and Settings have their own keys).
    w.set_hints(model(if s.panel == Panel::Main { hints } else { vec![] }));
}

/// A new receipt feeds down out of the slot, then its lines print in.
pub fn feed(w: &AppWindow) {
    w.set_feeding(false);
    w.set_fed(false);
    w.set_printed(false);
    let weak = w.as_weak();
    slint::Timer::single_shot(Duration::from_millis(30), move || {
        if let Some(w) = weak.upgrade() {
            w.set_feeding(true);
            w.set_fed(true);
            w.set_printed(true);
        }
    });
}

/// "The numbers" tabs in rows: up to four fit the panel's width; more go in two rows, the first
/// one the longer.
fn tab_rows(tabs: &[SharedString]) -> Vec<Vec<TabRef>> {
    let refs: Vec<TabRef> = tabs.iter().enumerate().map(|(i, tab)| TabRef { tab: tab.clone(), index: i as i32 }).collect();
    if refs.len() <= 4 {
        return vec![refs];
    }
    let first = refs.len().div_ceil(2);
    vec![refs[..first].to_vec(), refs[first..].to_vec()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn up_to_four_number_tabs_share_a_row_and_more_take_two() {
        let tabs = |n: usize| (0..n).map(|i| SharedString::from(format!("t{i}"))).collect::<Vec<_>>();
        let shape = |n: usize| tab_rows(&tabs(n)).iter().map(Vec::len).collect::<Vec<_>>();
        assert_eq!(shape(4), [4]);
        assert_eq!(shape(5), [3, 2]);
        assert_eq!(shape(6), [3, 3]);
        assert_eq!(tab_rows(&tabs(6))[1][0].index, 3, "each tab keeps its section's place");
    }

    #[test]
    fn italian_dates_take_the_article_their_day_needs() {
        // "il"/"al" before most days, "l'"/"all'" before 8 and 11 (otto, undici), and the 1st as "1°".
        assert_eq!(it_il("17 ottobre 2026"), "il 17 ottobre 2026");
        assert_eq!(it_al("17 ottobre 2026"), "al 17 ottobre 2026");
        assert_eq!(it_il("8 ottobre 2026"), "l'8 ottobre 2026");
        assert_eq!(it_al("8 ottobre 2026"), "all'8 ottobre 2026");
        assert_eq!(it_il("11 novembre 2026"), "l'11 novembre 2026");
        assert_eq!(it_al("11 novembre 2026"), "all'11 novembre 2026");
        assert_eq!(it_il("1 ottobre 2026"), "il 1° ottobre 2026");
        assert_eq!(it_al("1 ottobre 2026"), "al 1° ottobre 2026");
        // 18 and 81 aren't days with an article change ("il 18"); a date without a day stays as it is.
        assert_eq!(it_al("18 ottobre 2026"), "al 18 ottobre 2026");
        assert_eq!(it_il("ottobre 2026"), "il ottobre 2026");
    }
}
