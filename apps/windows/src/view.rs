//! Puts the state on the window: which views show, the receipt (built by fettle-receipt in the
//! viewer's language and time zone), the panel's texts and the status bar.

use crate::copy::t;
use crate::state::{Deep, MainView, Mat, Panel, PrivateCard, PrivateKind, ShareStatus, State, ThemeChoice};
use crate::{AppWindow, Copy, Group, Hint, Line, LinkRow, ReceiptData, Row, Section, Theme, sections};
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
            Panel::Main => match main {
                MainView::Home => "home",
                MainView::Checking => "checking",
                MainView::Result => "result",
            },
        }
        .into(),
    );
    w.set_mat(
        match s.mat() {
            Mat::Blank => "blank",
            Mat::Live => "live",
            Mat::Last => "last",
        }
        .into(),
    );
    w.set_alert(s.problem.map(|p| SharedString::from(t(p.key(), lang, &[]))).unwrap_or_default());

    let current = s.current.as_ref().and_then(|c| receipt_of(&c.report, lang, ctx).map(|r| (c, r)));
    let last = s.last.as_ref().and_then(|r| receipt_of(r, lang, ctx).map(|m| (r, m)));

    // The receipt on the mat: this check's, or the last one.
    match (s.mat(), &current, &last) {
        (Mat::Live, Some((_, (_, m))), _) | (Mat::Last, _, Some((_, (_, m)))) => {
            changed(&mut cache.receipt, format!("{:?}|{}|{}|{}", s.mat(), lang.code(), m.meta.collected_at, m.meta.when), || w.set_receipt(receipt_data(m, lang)));
        }
        _ => {}
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

    // After a check: the answer, the Deep check when it would show more, the numbers.
    if let Some((checked, (receipt, m))) = &current {
        w.set_answer(m.actions.first().cloned().unwrap_or_else(|| t("result.nothing", lang, &[])).into());
        w.set_deep(wants_deep_check(receipt, checked.report.elevated));
        changed(&mut cache.numbers, format!("{}|{}|{}", lang.code(), m.meta.collected_at, m.meta.when), || {
            w.set_numbers(model(sections_of(m, lang).into_iter().filter(|sec| slint::Model::row_count(&sec.details) > 0).collect()));
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
        w.set_share_delete_link(shared.delete_url.clone().into());
        w.set_share_expires(t("share.expires", lang, &[("date", long_local_date(&shared.expires_at).into())]).into());
    }
    if let Some(c) = &s.current {
        // Exactly what would leave this PC: the redacted report and the engine version.
        let outgoing = Report { engine_version: Some(fettle_verdicts::ENGINE_VERSION.to_string()), ..c.report.clone() };
        w.set_share_json(serde_json::to_string_pretty(&outgoing).unwrap_or_default().into());
    }
    w.set_share_note(s.share_note.map(|k| t(k, lang, &[])).unwrap_or_default().into());
    let links: Vec<LinkRow> = s.links.iter().map(|l| LinkRow { id: l.id.clone().into(), url: l.url.clone().into(), until: t("links.until", lang, &[("date", long_local_date(&l.expires_at).into())]).into() }).collect();
    changed(&mut cache.links, links.iter().map(|l| format!("{}={}", l.id, l.until)).collect::<Vec<_>>().join("|"), || w.set_links(model(links.clone())));
    w.set_links_label(t("title.links", lang, &[("count", Param::Int(s.links.len() as i64))]).into());
    w.set_links_status(s.links_status.map(|(k, _)| t(k, lang, &[])).unwrap_or_default().into());
    w.set_links_failed(s.links_status.is_some_and(|(_, failed)| failed));
    w.set_links_busy(s.deleting.is_some());

    // The status bar: what happened, which Windows, and the shortcuts that work right now.
    let (items, hints) = match (main, &current, &last) {
        (MainView::Checking, _, _) => (vec![t("status.checking", lang, &[])], vec![]),
        (MainView::Result, Some((c, _)), _) => {
            let secs = seconds(c.seconds, lang);
            let key = if c.report.elevated { "status.deep_time" } else { "status.quick_time" };
            (
                [Some(t(key, lang, &[("s", secs.into())])), windows_of(&c.report)].into_iter().flatten().collect(),
                vec![hint("Ctrl R", t("scan.again", lang, &[])), hint("Ctrl P", t("export.print", lang, &[]))],
            )
        }
        (_, _, Some((report, _))) => {
            let at = fmt::parse_rfc3339(&report.collected_at).unwrap_or(0);
            let local = at + i64::from((ctx.zone)(at)) * 60_000;
            let date = fmt::date_long(&fmt::utc_day(local), lang);
            ([Some(t("status.last", lang, &[("date", date.into())])), windows_of(report)].into_iter().flatten().collect(), vec![hint("Enter", t("scan.again", lang, &[]))])
        }
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
