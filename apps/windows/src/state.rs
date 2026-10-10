//! The app as a state machine: everything the window shows follows from `State`, and every change
//! goes through `update`, which returns the side effects to run (scan, save, relaunch…). No Slint
//! and no Windows here, so every rule is tested directly.

use crate::share::{Expiry, Shared};
use crate::store::SavedLink;
pub use crate::update::UpdateRecord;
use fettle_facts::Report;
use fettle_verdicts::Locale;
use serde::{Deserialize, Serialize};

/// The side panel's views. Main shows home, checking or the result, depending on the scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Main,
    Share,
    Links,
    Settings,
    /// The receipt's pages as they'll print, before Windows' print dialog.
    Preview,
}

/// What the main view shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainView {
    Home,
    Checking,
    Result,
}

/// What lies on the mat: the blank receipt, this check's receipt, or the last one, faded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mat {
    Blank,
    Live,
    Last,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    #[default]
    Auto,
    Light,
    Dark,
    Oled,
}

/// How often the update check asks the website.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UpdateEvery {
    Hourly,
    #[default]
    Daily,
    Weekly,
}

impl UpdateEvery {
    pub fn ms(self) -> i64 {
        match self {
            UpdateEvery::Hourly => 3_600_000,
            UpdateEvery::Daily => crate::update::DAY_MS,
            UpdateEvery::Weekly => 7 * crate::update::DAY_MS,
        }
    }
}

/// Kept on this PC only (settings.json). A missing or wrong-typed field takes its default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Keep the last check on this PC and show it, faded, at the next launch.
    pub remember: bool,
    /// Ask the website whether a new version exists.
    pub updates: bool,
    /// How often.
    pub update_every: UpdateEvery,
    /// Show this PC's name and serial numbers in the side panel (never in the report).
    pub show_private: bool,
    /// None: follow Windows.
    pub lang: Option<Locale>,
    pub theme: ThemeChoice,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { remember: true, updates: true, update_every: UpdateEvery::Daily, show_private: false, lang: None, theme: ThemeChoice::Auto }
    }
}

/// Why a check didn't finish (app string keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanProblem {
    Failed,
    Busy,
}

impl ScanProblem {
    pub fn key(&self) -> &'static str {
        match self {
            ScanProblem::Failed => "scan.failed",
            ScanProblem::Busy => "scan.busy",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Scan {
    Idle,
    Running,
    Done,
}

/// A password on its way to the server: never printed, logged or kept.
#[derive(Clone, PartialEq)]
pub struct Secret(pub String);

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(***)")
    }
}

/// The Share view: choosing, sending, the new link, or what went wrong (key, minutes to wait).
#[derive(Debug, Clone, PartialEq)]
pub enum ShareStatus {
    Editing,
    Sending,
    Done(Shared),
    Failed(&'static str, Option<i64>),
}

/// What the privacy switch shows: this PC's name and serial numbers, read on demand, held only in
/// the app's memory while the switch is on (never in State, storage, files or anything sent).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PrivateCard {
    pub pc_name: Option<String>,
    pub items: Vec<(PrivateKind, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrivateKind {
    System,
    Board,
    /// A disk, by its model when Windows names it.
    Disk(Option<String>),
}

/// This run's check: the redacted report and how long it took.
#[derive(Debug, Clone, PartialEq)]
pub struct Checked {
    pub report: Report,
    pub seconds: f64,
}

/// The Deep check button: idle, waiting for Windows' prompt, or what went wrong (an app string key).
#[derive(Debug, Clone, PartialEq)]
pub enum Deep {
    Idle,
    Asking,
    Problem(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeepError {
    Cancelled,
    Busy,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub panel: Panel,
    pub scan: Scan,
    /// Why the last check didn't finish, until the next one starts.
    pub problem: Option<ScanProblem>,
    pub done_sources: Vec<String>,
    pub current: Option<Checked>,
    pub last: Option<Report>,
    pub settings: Settings,
    pub share_busy: bool,
    /// The dock's last export message (an app string key), cleared by the next action.
    pub export: Option<&'static str>,
    pub deep: Deep,
    /// The privacy switch's details are loaded (they live in the app, never in State or storage).
    pub private_shown: bool,
    pub share: ShareStatus,
    /// The links this PC shared and can still delete.
    pub links: Vec<SavedLink>,
    /// The Links view's last message: (app string key, whether it's a failure).
    pub links_status: Option<(&'static str, bool)>,
    /// A link being deleted (its id): other Delete buttons wait.
    pub deleting: Option<String>,
    /// The links are being checked against the server (Delete waits meanwhile).
    pub checking_links: bool,
    /// How many links the last check found deleted on the website (for its message).
    pub links_gone: usize,
    /// The Share view's last message after the link was made (copied, or a failure).
    pub share_note: Option<&'static str>,
    /// Whether the last copy was the delete link (so the note says which link was copied).
    pub copied_delete: bool,
    /// When the website was last asked about updates, and what it said.
    pub update: UpdateRecord,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// What was stored on this PC, at launch.
    Loaded { settings: Settings, last: Option<Report>, links: Vec<SavedLink> },
    Start,
    SourceDone(String),
    Finished(Result<Report, ScanProblem>, f64),
    Open(Panel),
    Back,
    SetLang(Option<Locale>),
    SetTheme(ThemeChoice),
    SetRemember(bool),
    SetShowPrivate(bool),
    SetUpdates(bool),
    SetUpdateEvery(UpdateEvery),
    /// The details arrived; the app keeps the card, the state only notes it's shown.
    PrivateLoaded(PrivateCard),
    DeepAsked,
    DeepRefused(DeepError),
    /// The Deep check's report (elevated, with the user's own startup apps) and how long it took,
    /// or why there's none.
    DeepFinished(Result<Report, DeepError>, f64),
    SaveAsked,
    CopyAsked,
    PrintAsked,
    /// Print (or save the PDF) from the preview.
    PrintConfirmed,
    CreateLink { expires: Expiry, password: Option<Secret> },
    LinkCreated(Result<Shared, (&'static str, Option<i64>)>),
    CopyLink,
    /// Copy the delete link (shown only once, so it has its own Copy).
    CopyDeleteLink,
    OpenLink,
    LinkCopied(bool),
    DeleteLink(String),
    LinkDeleted(Result<(String, crate::share::Deleted), ()>),
    /// "Check which still work" in My shared links.
    CheckLinks,
    /// The ids the server said are gone (Err: it couldn't be asked).
    LinksChecked(Result<Vec<String>, ()>),
    Exported(Option<&'static str>),
    ShareBusy(bool),
    /// What this PC remembers of the last update check, at launch.
    UpdateRecord(UpdateRecord),
    /// Every so often while the app is open: a day may have passed.
    UpdateTick,
    /// The website's answer (None: no answer, or a bad one) and when.
    UpdateChecked(Option<crate::update::Latest>, i64),
    /// Download the newer version (its page, in the app's language).
    OpenUpdate,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    RunScan,
    SaveLast(Report),
    ClearLast,
    SaveSettings(Settings),
    /// Run the Deep check: the elevated helper's scan, merged with the user's own.
    DeepCheck,
    LoadPrivate,
    ForgetPrivate,
    /// Save this check (PDF or JSON, as the user picks), copy its markdown, or print it.
    Save,
    Copy,
    Print,
    /// Draw the pages for the preview.
    RenderPreview,
    /// Upload this check (with the engine version) as a link.
    Share { expires: Expiry, password: Option<Secret> },
    SaveLinks(Vec<SavedLink>),
    CopyText(String),
    OpenUrl(String),
    Delete(SavedLink),
    /// Ask the server which of these links still exist.
    CheckLinks(Vec<SavedLink>),
    /// Ask the website for the latest version, if a day has passed since the last answer.
    CheckUpdates,
    SaveUpdate(UpdateRecord),
    OpenUpdatePage,
}

impl Default for State {
    fn default() -> Self {
        State {
            panel: Panel::Main,
            scan: Scan::Idle,
            problem: None,
            done_sources: Vec::new(),
            current: None,
            last: None,
            settings: Settings::default(),
            share_busy: false,
            export: None,
            deep: Deep::Idle,
            private_shown: false,
            share: ShareStatus::Editing,
            links: Vec::new(),
            links_status: None,
            deleting: None,
            checking_links: false,
            links_gone: 0,
            share_note: None,
            copied_delete: false,
            update: UpdateRecord::default(),
        }
    }
}

impl State {
    /// The language in use: the chosen one, or Windows' for Auto.
    pub fn lang(&self, system: Locale) -> Locale {
        self.settings.lang.unwrap_or(system)
    }

    pub fn main_view(&self) -> MainView {
        match self.scan {
            Scan::Running => MainView::Checking,
            Scan::Done if self.current.is_some() => MainView::Result,
            _ => MainView::Home,
        }
    }

    pub fn mat(&self) -> Mat {
        match self.main_view() {
            MainView::Checking => Mat::Blank,
            MainView::Result => Mat::Live,
            MainView::Home if self.last.is_some() => Mat::Last,
            MainView::Home => Mat::Blank,
        }
    }

    /// A newer version to offer: only with updates on.
    pub fn update_note(&self) -> Option<&crate::update::Latest> {
        self.update.latest.as_ref().filter(|l| self.settings.updates && crate::update::newer(&l.version, crate::update::CURRENT))
    }

    fn check_updates(&self) -> Vec<Effect> {
        if self.settings.updates { vec![Effect::CheckUpdates] } else { vec![] }
    }

    fn save_settings(&self) -> Effect {
        Effect::SaveSettings(self.settings.clone())
    }

    pub fn update(&mut self, event: Event) -> Vec<Effect> {
        match event {
            Event::Loaded { settings, last, links } => {
                // A remembered check shows only while remembering is on.
                self.last = if settings.remember { last } else { None };
                self.settings = settings;
                self.links = links;
                vec![]
            }
            Event::Start => {
                // A Deep check is under way: its result will show in a moment.
                if self.scan == Scan::Running || self.deep == Deep::Asking {
                    return vec![];
                }
                self.scan = Scan::Running;
                self.problem = None;
                self.done_sources.clear();
                self.export = None;
                self.deep = Deep::Idle;
                self.private_shown = false;
                vec![Effect::RunScan]
            }
            Event::SourceDone(name) => {
                if self.scan == Scan::Running && !self.done_sources.contains(&name) {
                    self.done_sources.push(name);
                }
                vec![]
            }
            Event::Finished(Ok(report), seconds) => {
                self.scan = Scan::Done;
                self.current = Some(Checked { report: report.clone(), seconds });
                let mut effects = Vec::new();
                if self.settings.remember {
                    self.last = Some(report.clone());
                    effects.push(Effect::SaveLast(report));
                }
                if self.settings.show_private {
                    effects.push(Effect::LoadPrivate);
                }
                effects
            }
            Event::Finished(Err(problem), _) => {
                // A failed re-check keeps this run's receipt and says so above it; with no receipt
                // yet, the problem shows on the first screen (with the last receipt, if any).
                self.scan = if self.current.is_some() { Scan::Done } else { Scan::Idle };
                self.problem = Some(problem);
                vec![]
            }
            Event::Open(panel) => {
                // A view can't be left while it's sending; Share needs a receipt to share.
                if self.share_busy || (panel == Panel::Share && self.current.is_none()) {
                    return vec![];
                }
                if panel == Panel::Share {
                    self.share = ShareStatus::Editing;
                    self.share_note = None;
                }
                if panel == Panel::Links {
                    self.links_status = None;
                }
                self.panel = panel;
                vec![]
            }
            Event::Back => {
                if !self.share_busy {
                    self.panel = Panel::Main;
                }
                vec![]
            }
            Event::SetLang(lang) => {
                self.settings.lang = lang;
                vec![self.save_settings()]
            }
            Event::SetTheme(theme) => {
                self.settings.theme = theme;
                vec![self.save_settings()]
            }
            Event::SetRemember(on) => {
                self.settings.remember = on;
                let mut effects = vec![self.save_settings()];
                if on {
                    if let Some(c) = &self.current {
                        self.last = Some(c.report.clone());
                        effects.push(Effect::SaveLast(c.report.clone()));
                    }
                } else {
                    self.last = None;
                    effects.push(Effect::ClearLast);
                }
                effects
            }
            Event::SetShowPrivate(on) => {
                self.settings.show_private = on;
                let mut effects = vec![self.save_settings()];
                if on && self.current.is_some() {
                    effects.push(Effect::LoadPrivate);
                } else if !on {
                    self.private_shown = false;
                    effects.push(Effect::ForgetPrivate);
                }
                effects
            }
            Event::SetUpdates(on) => {
                self.settings.updates = on;
                let mut effects = vec![self.save_settings()];
                effects.extend(self.check_updates());
                effects
            }
            Event::PrivateLoaded(_) => {
                self.private_shown = self.settings.show_private;
                vec![]
            }
            Event::DeepAsked => {
                // While Windows' prompt is open, a second click would ask twice.
                if self.deep == Deep::Asking {
                    return vec![];
                }
                // A check is running: restarting now would throw it away.
                if self.scan == Scan::Running {
                    self.deep = Deep::Problem("scan.busy");
                    return vec![];
                }
                self.deep = Deep::Asking;
                vec![Effect::DeepCheck]
            }
            // Shown like any check, in this window.
            Event::DeepFinished(Ok(report), seconds) => {
                self.deep = Deep::Idle;
                self.update(Event::Finished(Ok(report), seconds))
            }
            Event::DeepFinished(Err(error), _) => self.update(Event::DeepRefused(error)),
            Event::DeepRefused(error) => {
                self.deep = Deep::Problem(match error {
                    DeepError::Cancelled => "deep.cancelled",
                    DeepError::Busy => "scan.busy",
                    DeepError::Failed => "deep.failed",
                });
                vec![]
            }
            // Print shows the pages first; printing happens from the preview.
            Event::PrintAsked => {
                if self.current.is_none() {
                    return vec![];
                }
                self.panel = Panel::Preview;
                vec![Effect::RenderPreview]
            }
            Event::PrintConfirmed => {
                if self.panel != Panel::Preview || self.current.is_none() {
                    return vec![];
                }
                self.panel = Panel::Main;
                self.export = None;
                vec![Effect::Print]
            }
            Event::SaveAsked | Event::CopyAsked => {
                if self.current.is_none() {
                    return vec![];
                }
                self.export = None;
                vec![if event == Event::SaveAsked { Effect::Save } else { Effect::Copy }]
            }
            Event::CreateLink { expires, password } => {
                if self.share == ShareStatus::Sending || self.current.is_none() {
                    return vec![];
                }
                self.share = ShareStatus::Sending;
                self.share_busy = true;
                vec![Effect::Share { expires, password }]
            }
            Event::LinkCreated(result) => {
                self.share_busy = false;
                match result {
                    Ok(shared) => {
                        // The app keeps the delete token so it can delete the link later.
                        self.links.push(SavedLink { id: shared.id.clone(), url: shared.url.clone(), delete_token: shared.delete_token.clone(), expires_at: shared.expires_at.clone() });
                        self.share = ShareStatus::Done(shared);
                        vec![Effect::SaveLinks(self.links.clone())]
                    }
                    Err((key, minutes)) => {
                        self.share = ShareStatus::Failed(key, minutes);
                        vec![]
                    }
                }
            }
            Event::CopyDeleteLink => {
                let ShareStatus::Done(shared) = &self.share else { return vec![] };
                self.share_note = None;
                self.copied_delete = true;
                vec![Effect::CopyText(shared.delete_url.clone())]
            }
            Event::CopyLink | Event::OpenLink => {
                let ShareStatus::Done(shared) = &self.share else { return vec![] };
                self.share_note = None;
                self.copied_delete = false;
                vec![if event == Event::CopyLink { Effect::CopyText(shared.url.clone()) } else { Effect::OpenUrl(shared.url.clone()) }]
            }
            Event::LinkCopied(ok) => {
                self.share_note = Some(match (ok, self.copied_delete) {
                    (false, _) => "export.failed",
                    (true, true) => "share.delete_copied",
                    (true, false) => "share.copied",
                });
                vec![]
            }
            Event::DeleteLink(id) => {
                if self.deleting.is_some() || self.checking_links {
                    return vec![];
                }
                let Some(link) = self.links.iter().find(|l| l.id == id).cloned() else { return vec![] };
                self.deleting = Some(id);
                self.links_status = None;
                vec![Effect::Delete(link)]
            }
            Event::LinkDeleted(result) => {
                self.deleting = None;
                match result {
                    Ok((id, how)) => {
                        self.links.retain(|l| l.id != id);
                        let key = if how == crate::share::Deleted::AlreadyGone { "links.already_gone" } else { "links.deleted" };
                        self.links_status = Some((key, false));
                        vec![Effect::SaveLinks(self.links.clone())]
                    }
                    Err(()) => {
                        self.links_status = Some(("links.delete_failed", true));
                        vec![]
                    }
                }
            }
            Event::CheckLinks => {
                if self.links.is_empty() || self.checking_links || self.deleting.is_some() {
                    return vec![];
                }
                self.checking_links = true;
                self.links_status = None;
                vec![Effect::CheckLinks(self.links.clone())]
            }
            Event::LinksChecked(result) => {
                self.checking_links = false;
                match result {
                    Ok(gone) => {
                        let before = self.links.len();
                        self.links.retain(|l| !gone.contains(&l.id));
                        self.links_gone = before - self.links.len();
                        self.links_status = Some((
                            match self.links_gone {
                                0 => "links.all_work",
                                1 => "links.one_gone",
                                _ => "links.some_gone",
                            },
                            false,
                        ));
                        if self.links_gone > 0 { vec![Effect::SaveLinks(self.links.clone())] } else { vec![] }
                    }
                    Err(()) => {
                        self.links_status = Some(("links.check_failed", true));
                        vec![]
                    }
                }
            }
            Event::Exported(key) => {
                self.export = key;
                vec![]
            }
            Event::ShareBusy(busy) => {
                self.share_busy = busy;
                vec![]
            }
            Event::SetUpdateEvery(every) => {
                self.settings.update_every = every;
                let mut effects = vec![self.save_settings()];
                effects.extend(self.check_updates());
                effects
            }
            Event::UpdateRecord(record) => {
                self.update = record;
                self.check_updates()
            }
            Event::UpdateTick => self.check_updates(),
            // No answer is no news, and isn't recorded: the website is asked again next time.
            Event::UpdateChecked(None, _) => vec![],
            Event::UpdateChecked(Some(latest), at) => {
                self.update = UpdateRecord { checked_at: at, latest: Some(latest) };
                vec![Effect::SaveUpdate(self.update.clone())]
            }
            Event::OpenUpdate => {
                if self.update_note().is_some() { vec![Effect::OpenUpdatePage] } else { vec![] }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fettle_facts::Facts;

    fn report(at: &str) -> Report {
        Report { schema_version: 1, app_version: "0.1.0".into(), engine_version: None, collected_at: at.into(), elevated: false, facts: Facts::default() }
    }

    fn checked_state() -> State {
        let mut s = State::default();
        s.update(Event::Start);
        s.update(Event::Finished(Ok(report("2026-10-09T10:00:00Z")), 0.7));
        s
    }

    #[test]
    fn first_run_shows_home_on_a_blank_receipt() {
        let s = State::default();
        assert_eq!((s.panel, s.main_view(), s.mat()), (Panel::Main, MainView::Home, Mat::Blank));
    }

    #[test]
    fn start_runs_one_check_at_a_time() {
        let mut s = State::default();
        assert_eq!(s.update(Event::Start), vec![Effect::RunScan]);
        assert_eq!((s.main_view(), s.mat()), (MainView::Checking, Mat::Blank));
        // A second click or Enter while the check runs does nothing.
        assert_eq!(s.update(Event::Start), vec![]);
    }

    #[test]
    fn sources_tick_while_checking() {
        let mut s = State::default();
        s.update(Event::Start);
        s.update(Event::SourceDone("cpu".into()));
        assert_eq!(s.done_sources, vec!["cpu".to_string()]);
        // A new check starts from nothing ticked.
        s.update(Event::Finished(Ok(report("2026-10-09T10:00:00Z")), 0.7));
        s.update(Event::Start);
        assert!(s.done_sources.is_empty());
    }

    #[test]
    fn a_finished_check_shows_its_receipt_and_is_remembered() {
        let mut s = State::default();
        s.update(Event::Start);
        let r = report("2026-10-09T10:00:00Z");
        assert_eq!(s.update(Event::Finished(Ok(r.clone()), 0.7)), vec![Effect::SaveLast(r.clone())]);
        assert_eq!((s.main_view(), s.mat()), (MainView::Result, Mat::Live));
        assert_eq!(s.last, Some(r));
    }

    #[test]
    fn nothing_is_remembered_when_remember_is_off() {
        let mut s = State::default();
        s.update(Event::Loaded { settings: Settings { remember: false, ..Settings::default() }, last: None, links: vec![] });
        s.update(Event::Start);
        assert_eq!(s.update(Event::Finished(Ok(report("2026-10-09T10:00:00Z")), 0.7)), vec![]);
        assert_eq!(s.last, None);
    }

    #[test]
    fn a_failed_first_check_goes_back_home_and_says_so() {
        let mut s = State::default();
        s.update(Event::Loaded { settings: Settings::default(), last: Some(report("2026-10-08T10:00:00Z")), links: vec![] });
        s.update(Event::Start);
        s.update(Event::Finished(Err(ScanProblem::Failed), 1.0));
        assert_eq!(s.main_view(), MainView::Home);
        assert_eq!(s.problem, Some(ScanProblem::Failed));
        // The last good receipt waits, faded.
        assert_eq!(s.mat(), Mat::Last);
    }

    #[test]
    fn a_failed_re_check_keeps_this_runs_receipt_and_says_so() {
        let mut s = State::default();
        s.update(Event::Loaded { settings: Settings { remember: false, ..Settings::default() }, last: None, links: vec![] });
        s.update(Event::Start);
        s.update(Event::Finished(Ok(report("2026-10-09T10:00:00Z")), 0.7));
        s.update(Event::Start);
        s.update(Event::Finished(Err(ScanProblem::Busy), 1.0));
        assert_eq!((s.main_view(), s.mat()), (MainView::Result, Mat::Live));
        assert_eq!(s.problem, Some(ScanProblem::Busy));
        assert_eq!(s.update(Event::SaveAsked), vec![Effect::Save]);
        // The next check clears the problem.
        s.update(Event::Start);
        assert_eq!(s.problem, None);
    }

    #[test]
    fn coming_back_shows_the_last_receipt_faded() {
        let mut s = State::default();
        s.update(Event::Loaded { settings: Settings::default(), last: Some(report("2026-10-08T10:00:00Z")), links: vec![] });
        assert_eq!((s.main_view(), s.mat()), (MainView::Home, Mat::Last));
    }

    #[test]
    fn turning_remember_off_forgets_the_last_check_at_once() {
        let mut s = checked_state();
        let effects = s.update(Event::SetRemember(false));
        assert!(effects.contains(&Effect::ClearLast), "{effects:?}");
        assert!(effects.iter().any(|e| matches!(e, Effect::SaveSettings(st) if !st.remember)));
        assert_eq!(s.last, None);
    }

    #[test]
    fn turning_remember_on_keeps_this_check() {
        let mut s = State::default();
        s.update(Event::Loaded { settings: Settings { remember: false, ..Settings::default() }, last: None, links: vec![] });
        s.update(Event::Start);
        let r = report("2026-10-09T10:00:00Z");
        s.update(Event::Finished(Ok(r.clone()), 0.7));
        assert!(s.update(Event::SetRemember(true)).contains(&Effect::SaveLast(r)));
    }

    #[test]
    fn views_open_from_the_panel_and_back_returns_to_main() {
        let mut s = checked_state();
        s.update(Event::Open(Panel::Settings));
        assert_eq!(s.panel, Panel::Settings);
        s.update(Event::Back);
        assert_eq!((s.panel, s.main_view()), (Panel::Main, MainView::Result));
    }

    #[test]
    fn views_stay_put_while_a_link_is_being_created() {
        let mut s = checked_state();
        s.update(Event::Open(Panel::Share));
        s.update(Event::ShareBusy(true));
        s.update(Event::Open(Panel::Settings));
        s.update(Event::Back);
        assert_eq!(s.panel, Panel::Share);
        s.update(Event::ShareBusy(false));
        s.update(Event::Back);
        assert_eq!(s.panel, Panel::Main);
    }

    #[test]
    fn the_privacy_switch_loads_and_forgets_the_details() {
        let mut s = checked_state();
        let on = s.update(Event::SetShowPrivate(true));
        assert!(on.contains(&Effect::LoadPrivate), "{on:?}");
        s.update(Event::PrivateLoaded(PrivateCard::default()));
        assert!(s.private_shown);
        let off = s.update(Event::SetShowPrivate(false));
        assert!(off.contains(&Effect::ForgetPrivate), "{off:?}");
        assert!(!s.private_shown);
        // With the switch on, each new check reads the details again.
        s.update(Event::SetShowPrivate(true));
        s.update(Event::Start);
        assert!(s.update(Event::Finished(Ok(report("2026-10-09T11:00:00Z")), 0.7)).contains(&Effect::LoadPrivate));
    }

    #[test]
    fn language_and_theme_are_saved() {
        let mut s = State::default();
        assert_eq!(s.lang(Locale::It), Locale::It);
        assert!(matches!(s.update(Event::SetLang(Some(Locale::En))).as_slice(), [Effect::SaveSettings(st)] if st.lang == Some(Locale::En)));
        assert_eq!(s.lang(Locale::It), Locale::En);
        assert!(matches!(s.update(Event::SetTheme(ThemeChoice::Oled)).as_slice(), [Effect::SaveSettings(st)] if st.theme == ThemeChoice::Oled));
    }

    #[test]
    fn the_deep_check_asks_once_and_says_what_went_wrong() {
        let mut s = checked_state();
        assert_eq!(s.update(Event::DeepAsked), vec![Effect::DeepCheck]);
        assert_eq!(s.deep, Deep::Asking);
        // A second click while Windows' prompt is open asks nothing more.
        assert_eq!(s.update(Event::DeepAsked), vec![]);
        s.update(Event::DeepRefused(DeepError::Cancelled));
        assert_eq!(s.deep, Deep::Problem("deep.cancelled"));
        s.update(Event::DeepAsked);
        s.update(Event::DeepRefused(DeepError::Busy));
        assert_eq!(s.deep, Deep::Problem("scan.busy"));
    }

    fn latest(version: &str) -> crate::update::Latest {
        crate::update::Latest { version: version.into(), page_en: "https://fettle.page/download".into(), page_it: "https://fettle.page/it/download".into() }
    }

    #[test]
    fn with_updates_on_the_app_asks_at_launch_and_every_so_often() {
        let mut s = State::default();
        assert_eq!(s.update(Event::UpdateRecord(UpdateRecord::default())), vec![Effect::CheckUpdates]);
        assert_eq!(s.update(Event::UpdateTick), vec![Effect::CheckUpdates]);
        s.update(Event::SetUpdates(false));
        assert_eq!(s.update(Event::UpdateTick), vec![]);
        assert!(s.update(Event::SetUpdates(true)).contains(&Effect::CheckUpdates));
    }

    #[test]
    fn how_often_to_ask_is_a_setting_and_asking_follows_it() {
        let mut s = State::default();
        assert_eq!(s.settings.update_every, UpdateEvery::Daily);
        let effects = s.update(Event::SetUpdateEvery(UpdateEvery::Weekly));
        assert!(matches!(effects.as_slice(), [Effect::SaveSettings(st), Effect::CheckUpdates] if st.update_every == UpdateEvery::Weekly), "{effects:?}");
    }

    #[test]
    fn with_updates_off_nothing_is_asked() {
        let mut s = State::default();
        s.update(Event::Loaded { settings: Settings { updates: false, ..Settings::default() }, last: None, links: vec![] });
        assert_eq!(s.update(Event::UpdateRecord(UpdateRecord::default())), vec![]);
    }

    #[test]
    fn a_newer_version_shows_a_note_until_updates_are_turned_off() {
        let mut s = State::default();
        let found = s.update(Event::UpdateChecked(Some(latest("999.0.0")), 1_000));
        assert_eq!(found, vec![Effect::SaveUpdate(UpdateRecord { checked_at: 1_000, latest: Some(latest("999.0.0")) })]);
        assert_eq!(s.update_note().map(|l| l.version.as_str()), Some("999.0.0"));
        assert_eq!(s.update(Event::OpenUpdate), vec![Effect::OpenUpdatePage]);
        s.update(Event::SetUpdates(false));
        assert_eq!(s.update_note(), None);
        assert_eq!(s.update(Event::OpenUpdate), vec![]);
    }

    #[test]
    fn the_same_version_or_a_failed_check_says_nothing() {
        let mut s = State::default();
        s.update(Event::UpdateChecked(Some(latest(crate::update::CURRENT)), 1_000));
        assert_eq!(s.update_note(), None);
        assert_eq!(s.update(Event::UpdateChecked(None, 2_000)), vec![], "a failed check isn't recorded: it's tried again next time");
        assert_eq!(s.update.checked_at, 1_000);
    }

    #[test]
    fn the_deep_check_shows_its_receipt_in_this_window() {
        let mut s = checked_state();
        s.update(Event::DeepAsked);
        let mut deep = report("2026-10-09T11:00:00Z");
        deep.elevated = true;
        let effects = s.update(Event::DeepFinished(Ok(deep.clone()), 1.4));
        assert_eq!((s.main_view(), s.mat()), (MainView::Result, Mat::Live));
        assert_eq!(s.current.as_ref().map(|c| c.report.clone()), Some(deep.clone()));
        assert_eq!(s.deep, Deep::Idle);
        assert!(effects.contains(&Effect::SaveLast(deep)), "{effects:?}");
        // A refused or failed one keeps the quick receipt and says why.
        s.update(Event::DeepAsked);
        s.update(Event::DeepFinished(Err(DeepError::Cancelled), 0.0));
        assert_eq!(s.deep, Deep::Problem("deep.cancelled"));
    }

    #[test]
    fn a_deep_check_waits_for_a_running_check() {
        let mut s = checked_state();
        s.update(Event::Start);
        assert_eq!(s.update(Event::DeepAsked), vec![], "restarting now would throw the check away");
        assert_eq!(s.deep, Deep::Problem("scan.busy"));
    }

    #[test]
    fn exports_need_a_receipt() {
        let mut s = State::default();
        assert_eq!(s.update(Event::SaveAsked), vec![]);
        let mut s = checked_state();
        s.update(Event::Exported(Some("export.saved")));
        assert_eq!(s.update(Event::CopyAsked), vec![Effect::Copy]);
        assert_eq!(s.export, None, "the old message goes while the new action runs");
        assert_eq!(s.update(Event::SaveAsked), vec![Effect::Save]);
        assert_eq!(s.update(Event::PrintAsked), vec![Effect::RenderPreview]);
    }

    #[test]
    fn print_opens_the_preview_and_printing_happens_from_there() {
        let mut s = checked_state();
        assert_eq!(s.update(Event::PrintAsked), vec![Effect::RenderPreview]);
        assert_eq!(s.panel, Panel::Preview);
        assert_eq!(s.update(Event::PrintConfirmed), vec![Effect::Print]);
        assert_eq!(s.panel, Panel::Main, "back to the receipt, where the dock says how it went");
    }

    #[test]
    fn the_preview_can_be_left_without_printing() {
        let mut s = checked_state();
        s.update(Event::PrintAsked);
        s.update(Event::Back);
        assert_eq!(s.panel, Panel::Main);
        assert_eq!(s.update(Event::PrintConfirmed), vec![], "nothing prints from outside the preview");
        assert_eq!(State::default().update(Event::PrintAsked), vec![], "no receipt, no preview");
    }

    fn shared(id: &str) -> Shared {
        Shared { id: id.into(), url: format!("https://fettle.page/r/{id}"), delete_url: format!("https://fettle.page/r/{id}/delete#t"), delete_token: "t".into(), expires_at: "2026-11-08T10:00:00Z".into(), locked: false }
    }

    #[test]
    fn either_link_can_be_copied_and_the_note_says_which() {
        let mut s = checked_state();
        s.update(Event::CreateLink { expires: crate::share::Expiry::Week, password: None });
        s.update(Event::LinkCreated(Ok(shared("m1"))));
        assert_eq!(s.update(Event::CopyDeleteLink), vec![Effect::CopyText("https://fettle.page/r/m1/delete#t".into())]);
        s.update(Event::LinkCopied(true));
        assert_eq!(s.share_note, Some("share.delete_copied"));
        assert_eq!(s.update(Event::CopyLink), vec![Effect::CopyText("https://fettle.page/r/m1".into())]);
        s.update(Event::LinkCopied(true));
        assert_eq!(s.share_note, Some("share.copied"));
    }

    #[test]
    fn a_link_is_created_once_and_remembered_for_deleting() {
        let mut s = checked_state();
        s.update(Event::Open(Panel::Share));
        let ask = Event::CreateLink { expires: Expiry::Month, password: Some(Secret("hunter22".into())) };
        assert!(matches!(s.update(ask.clone()).as_slice(), [Effect::Share { expires: Expiry::Month, .. }]));
        assert_eq!(s.update(ask), vec![], "a second click while sending");
        assert!(s.share_busy);
        let effects = s.update(Event::LinkCreated(Ok(shared("abc"))));
        assert_eq!(s.share, ShareStatus::Done(shared("abc")));
        assert!(!s.share_busy);
        assert_eq!(s.links.len(), 1);
        assert!(matches!(effects.as_slice(), [Effect::SaveLinks(l)] if l.len() == 1 && l[0].delete_token == "t"));
    }

    #[test]
    fn a_failed_link_says_why_and_can_be_tried_again() {
        let mut s = checked_state();
        s.update(Event::Open(Panel::Share));
        s.update(Event::CreateLink { expires: Expiry::Week, password: None });
        s.update(Event::LinkCreated(Err(("share.e.rate_limited_in", Some(2)))));
        assert_eq!(s.share, ShareStatus::Failed("share.e.rate_limited_in", Some(2)));
        assert!(matches!(s.update(Event::CreateLink { expires: Expiry::Week, password: None }).as_slice(), [Effect::Share { .. }]));
    }

    #[test]
    fn the_password_never_shows_in_debug_output() {
        let e = Event::CreateLink { expires: Expiry::Year, password: Some(Secret("hunter22".into())) };
        assert!(!format!("{e:?}").contains("hunter22"));
    }

    #[test]
    fn share_needs_a_receipt() {
        let mut s = State::default();
        s.update(Event::Open(Panel::Share));
        assert_eq!(s.panel, Panel::Main);
    }

    #[test]
    fn links_are_deleted_one_at_a_time() {
        let mut s = checked_state();
        s.update(Event::Open(Panel::Share));
        s.update(Event::CreateLink { expires: Expiry::Week, password: None });
        s.update(Event::LinkCreated(Ok(shared("a"))));
        s.update(Event::Open(Panel::Share));
        s.update(Event::CreateLink { expires: Expiry::Week, password: None });
        s.update(Event::LinkCreated(Ok(shared("b"))));
        s.update(Event::Open(Panel::Links));
        assert!(matches!(s.update(Event::DeleteLink("a".into())).as_slice(), [Effect::Delete(l)] if l.id == "a"));
        assert_eq!(s.update(Event::DeleteLink("b".into())), vec![], "one at a time");
        s.update(Event::LinkDeleted(Ok(("a".into(), crate::share::Deleted::Now))));
        assert_eq!(s.links.iter().map(|l| l.id.as_str()).collect::<Vec<_>>(), vec!["b"]);
        assert_eq!(s.links_status, Some(("links.deleted", false)));
        s.update(Event::DeleteLink("b".into()));
        s.update(Event::LinkDeleted(Err(())));
        assert_eq!(s.links.len(), 1, "kept: it may still be public");
        assert_eq!(s.links_status, Some(("links.delete_failed", true)));
    }

    fn with_links(ids: &[&str]) -> State {
        let mut s = checked_state();
        for id in ids {
            s.update(Event::Open(Panel::Share));
            s.update(Event::CreateLink { expires: Expiry::Week, password: None });
            s.update(Event::LinkCreated(Ok(shared(id))));
        }
        s.update(Event::Open(Panel::Links));
        s
    }

    #[test]
    fn checking_drops_the_links_deleted_on_the_website() {
        let mut s = with_links(&["a", "b", "c"]);
        let asked = s.update(Event::CheckLinks);
        assert!(matches!(asked.as_slice(), [Effect::CheckLinks(l)] if l.len() == 3), "{asked:?}");
        assert_eq!(s.update(Event::CheckLinks), vec![], "one check at a time");
        assert!(s.update(Event::DeleteLink("a".into())).is_empty(), "no deleting while checking");
        let done = s.update(Event::LinksChecked(Ok(vec!["a".into(), "c".into()])));
        assert_eq!(s.links.iter().map(|l| l.id.as_str()).collect::<Vec<_>>(), vec!["b"]);
        assert!(matches!(done.as_slice(), [Effect::SaveLinks(l)] if l.len() == 1));
        assert_eq!((s.links_status, s.links_gone), (Some(("links.some_gone", false)), 2));
        s.update(Event::CheckLinks);
        assert_eq!(s.update(Event::LinksChecked(Ok(vec![]))), vec![], "nothing gone, nothing to save");
        assert_eq!(s.links_status, Some(("links.all_work", false)));
        s.update(Event::CheckLinks);
        s.update(Event::LinksChecked(Ok(vec!["b".into()])));
        assert_eq!(s.links_status, Some(("links.one_gone", false)));
        assert_eq!(State::default().update(Event::CheckLinks), vec![], "nothing to check");
    }

    #[test]
    fn a_check_that_cant_reach_the_server_keeps_every_link() {
        let mut s = with_links(&["a"]);
        s.update(Event::CheckLinks);
        assert_eq!(s.update(Event::LinksChecked(Err(()))), vec![]);
        assert_eq!(s.links.len(), 1);
        assert_eq!(s.links_status, Some(("links.check_failed", true)));
    }

    #[test]
    fn deleting_a_link_already_deleted_on_the_website_says_so() {
        let mut s = with_links(&["a"]);
        s.update(Event::DeleteLink("a".into()));
        s.update(Event::LinkDeleted(Ok(("a".into(), crate::share::Deleted::AlreadyGone))));
        assert!(s.links.is_empty());
        assert_eq!(s.links_status, Some(("links.already_gone", false)));
    }

    #[test]
    fn an_export_message_lasts_until_the_next_check() {
        let mut s = checked_state();
        s.update(Event::Exported(Some("export.saved")));
        assert_eq!(s.export, Some("export.saved"));
        s.update(Event::Start);
        assert_eq!(s.export, None);
    }
}
