//! The app as a state machine: everything the window shows follows from `State`, and every change
//! goes through `update`, which returns the side effects to run (scan, save, relaunch…). No Slint
//! and no Windows here, so every rule is tested directly.

use crate::share::{Expiry, Shared};
pub use crate::store::SavedCheck;
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
    /// My checks: the checks saved on this PC.
    Checks,
    /// Two saved checks side by side (`State::compare`).
    Compare,
}

/// What the main view shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainView {
    Home,
    Checking,
    Result,
    /// A past check opened from My checks (`State::opened`).
    Past,
}

/// What lies on the mat: the blank receipt, this check's receipt, the last one (faded, at launch),
/// or a past check opened from My checks (faded, with its own date).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mat {
    Blank,
    Live,
    Last,
    Past,
}

/// The technician's details for the top of printouts (only ever on paper). Empty fields are left out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Technician {
    pub business: String,
    pub name: String,
    pub contact: String,
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
    /// Keep the last ten checks on this PC (My checks) and show the newest, faded, at the next launch.
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
    /// Your details on printouts.
    pub technician: Technician,
    /// Whether the user has chosen about the update check in Settings (or their settings say so).
    /// Until then "updates" isn't saved, so the PC's default (an all-users install's) still decides.
    #[serde(skip)]
    pub updates_chosen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { remember: true, updates: true, update_every: UpdateEvery::Daily, show_private: false, lang: None, theme: ThemeChoice::Auto, technician: Technician::default(), updates_chosen: false }
    }
}

/// Why a check didn't finish (app string keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanProblem {
    Failed,
}

impl ScanProblem {
    pub fn key(&self) -> &'static str {
        match self {
            ScanProblem::Failed => "scan.failed",
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

/// This run's check: the redacted report, how long it took, and the client and notes for its
/// printout (also saved with its entry while Remember is on).
#[derive(Debug, Clone, PartialEq)]
pub struct Checked {
    pub report: Report,
    pub seconds: f64,
    pub client: String,
    pub notes: String,
}

impl Checked {
    fn saved(&self) -> SavedCheck {
        SavedCheck { report: self.report.clone(), client: self.client.clone(), notes: self.notes.clone() }
    }
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
    Failed,
    /// Fettle's folder can't take the helper's report (a network share, say): trying again won't help.
    Folder,
}

#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub panel: Panel,
    pub scan: Scan,
    /// Why the last check didn't finish, until the next one starts.
    pub problem: Option<ScanProblem>,
    pub done_sources: Vec<String>,
    pub current: Option<Checked>,
    /// The checks saved on this PC, newest first (empty while Remember is off).
    pub checks: Vec<SavedCheck>,
    /// The past check opened from My checks (its id): it lies on the mat, and Save, Copy, Print and
    /// Share act on it (`shown_report`).
    pub opened: Option<String>,
    /// The two checks being compared (their ids, older first when chosen that way). The comparison
    /// itself is made where it's shown.
    pub compare: Option<(String, String)>,
    /// A logo for printouts is saved (the app reads the PNG when it needs it).
    pub has_logo: bool,
    /// Why the last logo chosen can't be used (an app string key), until the next try, a logo that
    /// works, or Settings opening again.
    pub logo_problem: Option<&'static str>,
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
    /// The Links view's last message (an app string key).
    pub links_status: Option<&'static str>,
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
    Loaded { settings: Settings, checks: Vec<SavedCheck>, links: Vec<SavedLink>, has_logo: bool },
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
    /// The saved links as they are now, other windows' included.
    LinksLoaded(Vec<SavedLink>),
    Exported(Option<&'static str>),
    /// What this PC remembers of the last update check, at launch.
    UpdateRecord(UpdateRecord),
    /// Every so often while the app is open: a day may have passed.
    UpdateTick,
    /// The website's answer (None: no answer, or a bad one) and when.
    UpdateChecked(UpdateRecord),
    /// Download the newer version (its page, in the app's language).
    OpenUpdate,
    /// The saved checks as they are now, other windows' included.
    ChecksLoaded(Vec<SavedCheck>),
    /// Put this saved check (its id) on the mat.
    OpenCheck(String),
    /// Back to this run's receipt (or the first screen).
    CloseCheck,
    DeleteCheck(String),
    /// Compare these two saved checks (ids).
    CompareChecks(String, String),
    /// After a check: compare it with the saved one before it.
    CompareWithPrevious,
    /// The printout's Client and Notes, for the check shown.
    SetClient(String),
    SetNotes(String),
    /// Your details on printouts.
    SetTechnician(Technician),
    /// A logo for printouts, already prepared as a PNG.
    SetLogo(Vec<u8>),
    RemoveLogo,
    /// "Choose…" beside the logo: Windows' open dialog.
    ChooseLogo,
    /// The picture chosen can't be the logo (an app string key says why).
    LogoRefused(&'static str),
    /// Copy the comparison shown as text.
    CopyComparison,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    RunScan,
    /// Save this check beside the ones already saved (other windows' included).
    AddCheck(Box<SavedCheck>),
    DeleteCheck(String),
    /// Forget every saved check (Remember turned off).
    ClearChecks,
    /// Read the saved checks again (another window may have added or deleted some).
    ReloadChecks,
    /// Save the client and notes with this check's entry.
    SaveCheckNotes { id: String, client: String, notes: String },
    SaveLogo(Vec<u8>),
    ClearLogo,
    /// Ask for a picture, make it ready (`print::logo::prepare`) and answer SetLogo or LogoRefused.
    ChooseLogo,
    /// Copy the comparison of `State::compared` as text.
    CopyComparison,
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
    /// Save a new link beside the ones already saved (other windows' included).
    AddLink(SavedLink),
    /// Forget these links, keeping every other one saved.
    ForgetLinks(Vec<String>),
    /// Read the saved links again (another window may have shared or deleted some).
    ReloadLinks,
    CopyText(String),
    OpenUrl(String),
    Delete(SavedLink),
    /// Ask the server which of these links still exist.
    CheckLinks(Vec<SavedLink>),
    /// Ask the website for the latest version, if the chosen interval has passed since the last answer.
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
            checks: Vec::new(),
            opened: None,
            compare: None,
            has_logo: false,
            logo_problem: None,
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
            _ if self.opened_check().is_some() => MainView::Past,
            Scan::Done if self.current.is_some() => MainView::Result,
            _ => MainView::Home,
        }
    }

    pub fn mat(&self) -> Mat {
        match self.main_view() {
            MainView::Checking => Mat::Blank,
            MainView::Past => Mat::Past,
            MainView::Result => Mat::Live,
            MainView::Home if self.last_check().is_some() => Mat::Last,
            MainView::Home => Mat::Blank,
        }
    }

    /// The newest saved check: the faded receipt at launch.
    pub fn last_check(&self) -> Option<&Report> {
        self.checks.first().map(|c| &c.report)
    }

    /// The past check opened from My checks, while it's still saved.
    pub fn opened_check(&self) -> Option<&SavedCheck> {
        let id = self.opened.as_deref()?;
        self.checks.iter().find(|c| c.id() == id)
    }

    /// The receipt Save, Copy, Print and Share act on: the opened past check, else this run's.
    pub fn shown_report(&self) -> Option<&Report> {
        match self.opened_check() {
            Some(c) => Some(&c.report),
            None => self.current.as_ref().map(|c| &c.report),
        }
    }

    /// The printout's Client and Notes for the receipt shown.
    pub fn shown_notes(&self) -> (&str, &str) {
        match (self.opened_check(), &self.current) {
            (Some(c), _) => (&c.client, &c.notes),
            (None, Some(c)) => (&c.client, &c.notes),
            (None, None) => ("", ""),
        }
    }

    /// A check by id: a saved one, or this run's.
    pub fn check_report(&self, id: &str) -> Option<&Report> {
        self.checks.iter().find(|c| c.id() == id).map(|c| &c.report).or(self.current.as_ref().map(|c| &c.report).filter(|r| r.collected_at == id))
    }

    /// After a check: the newest saved check made before it, for "Compare with the previous check".
    pub fn previous_check(&self) -> Option<&SavedCheck> {
        let current = self.current.as_ref()?;
        let at = fettle_verdicts::fmt::parse_rfc3339(&current.report.collected_at)?;
        self.checks.iter().find(|c| c.id() != current.report.collected_at && c.at().is_some_and(|t| t < at))
    }

    /// The two reports being compared, in the order chosen (`fettle_receipt::compare` sorts them).
    pub fn compared(&self) -> Option<(&Report, &Report)> {
        let (a, b) = self.compare.as_ref()?;
        Some((self.check_report(a)?, self.check_report(b)?))
    }

    /// Ends a comparison whose checks are gone.
    fn drop_stale_compare(&mut self) {
        if self.compare.is_some() && self.compared().is_none() {
            self.compare = None;
            if self.panel == Panel::Compare {
                self.panel = Panel::Main;
            }
        }
    }

    fn start_compare(&mut self, a: String, b: String) {
        if self.share_busy || a == b || self.check_report(&a).is_none() || self.check_report(&b).is_none() {
            return;
        }
        self.compare = Some((a, b));
        self.panel = Panel::Compare;
        // The receipt's last message doesn't follow into the comparison.
        self.export = None;
    }

    /// Sets the Client or Notes of the receipt shown; saved with its entry when it has one.
    fn set_notes(&mut self, client: Option<String>, notes: Option<String>) -> Vec<Effect> {
        let Some(id) = self.opened_check().map(|c| c.id().to_string()).or(self.current.as_ref().map(|c| c.report.collected_at.clone())) else { return vec![] };
        // Leaving a field always says so; only a change is saved.
        let (now_client, now_notes) = self.shown_notes();
        if client.as_ref().is_none_or(|c| c == now_client) && notes.as_ref().is_none_or(|n| n == now_notes) {
            return vec![];
        }
        let apply = |c: &mut String, n: &mut String| {
            if let Some(v) = &client {
                *c = v.clone();
            }
            if let Some(v) = &notes {
                *n = v.clone();
            }
        };
        if let Some(cur) = self.current.as_mut().filter(|c| c.report.collected_at == id) {
            apply(&mut cur.client, &mut cur.notes);
        }
        let Some(entry) = self.checks.iter_mut().find(|c| c.id() == id) else { return vec![] };
        apply(&mut entry.client, &mut entry.notes);
        vec![Effect::SaveCheckNotes { id, client: entry.client.clone(), notes: entry.notes.clone() }]
    }

    /// Saves a check in the list (and on this PC).
    fn remember(&mut self, check: SavedCheck) -> Effect {
        let mut checks = vec![check.clone()];
        checks.append(&mut self.checks);
        self.checks = crate::store::tidy_checks(checks);
        Effect::AddCheck(Box::new(check))
    }

    /// A newer version to offer: only with updates on.
    pub fn update_note(&self) -> Option<&crate::update::Latest> {
        self.update.latest.as_ref().filter(|l| self.settings.updates && crate::update::newer(&l.version, crate::update::CURRENT))
    }

    fn check_updates(&self) -> Vec<Effect> {
        if self.settings.updates { vec![Effect::CheckUpdates] } else { vec![] }
    }

    /// What a new check clears from the last one: its problem, its progress, the dock's message, the
    /// Deep check's state and the private card (read again for the new check).
    fn clear_for_a_new_check(&mut self) {
        self.problem = None;
        self.done_sources.clear();
        self.export = None;
        self.deep = Deep::Idle;
        self.private_shown = false;
        // The new check takes the mat from a past one.
        self.opened = None;
    }

    fn save_settings(&self) -> Effect {
        Effect::SaveSettings(self.settings.clone())
    }

    pub fn update(&mut self, event: Event) -> Vec<Effect> {
        match event {
            Event::Loaded { settings, checks, links, has_logo } => {
                // Saved checks show only while remembering is on.
                self.checks = if settings.remember { crate::store::tidy_checks(checks) } else { Vec::new() };
                self.settings = settings;
                self.links = links;
                self.has_logo = has_logo;
                vec![]
            }
            Event::Start => {
                // A Deep check is under way: its result will show in a moment.
                if self.scan == Scan::Running || self.deep == Deep::Asking {
                    return vec![];
                }
                self.scan = Scan::Running;
                self.clear_for_a_new_check();
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
                let checked = Checked { report, seconds, client: String::new(), notes: String::new() };
                let mut effects = Vec::new();
                if self.settings.remember {
                    effects.push(self.remember(checked.saved()));
                }
                self.current = Some(checked);
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
                // A view can't be left while it's sending; Share needs a receipt to share, Compare
                // two checks (CompareChecks opens it).
                if self.share_busy || (panel == Panel::Share && self.shown_report().is_none()) || (panel == Panel::Compare && self.compared().is_none()) {
                    return vec![];
                }
                if panel == Panel::Share {
                    self.share = ShareStatus::Editing;
                    self.share_note = None;
                }
                if panel == Panel::Settings {
                    self.logo_problem = None;
                }
                self.panel = panel;
                if panel == Panel::Links {
                    self.links_status = None;
                    return vec![Effect::ReloadLinks];
                }
                if panel == Panel::Checks {
                    return vec![Effect::ReloadChecks];
                }
                vec![]
            }
            Event::Back => {
                if !self.share_busy {
                    if self.panel == Panel::Compare {
                        self.compare = None;
                    }
                    self.panel = Panel::Main;
                }
                vec![]
            }
            Event::ChecksLoaded(checks) => {
                if self.settings.remember {
                    self.checks = crate::store::tidy_checks(checks);
                }
                // Deleted in another window meanwhile: no longer shown or compared.
                if self.opened_check().is_none() {
                    self.opened = None;
                }
                self.drop_stale_compare();
                vec![]
            }
            Event::OpenCheck(id) => {
                // A running check is about to take the mat.
                if self.scan == Scan::Running {
                    return vec![];
                }
                if self.current.as_ref().is_some_and(|c| c.report.collected_at == id) {
                    // This run's own check: just show it.
                    self.opened = None;
                } else if self.checks.iter().any(|c| c.id() == id) {
                    self.opened = Some(id);
                } else {
                    return vec![];
                }
                self.panel = Panel::Main;
                self.export = None;
                vec![]
            }
            Event::CloseCheck => {
                self.opened = None;
                self.export = None;
                vec![]
            }
            Event::DeleteCheck(id) => {
                if !self.checks.iter().any(|c| c.id() == id) {
                    return vec![];
                }
                self.checks.retain(|c| c.id() != id);
                if self.opened.as_deref() == Some(id.as_str()) {
                    self.opened = None;
                }
                self.drop_stale_compare();
                vec![Effect::DeleteCheck(id)]
            }
            Event::CompareChecks(a, b) => {
                self.start_compare(a, b);
                vec![]
            }
            Event::CompareWithPrevious => {
                if let (Some(before), Some(after)) = (self.previous_check().map(|c| c.id().to_string()), self.current.as_ref().map(|c| c.report.collected_at.clone())) {
                    self.start_compare(before, after);
                }
                vec![]
            }
            Event::SetClient(client) => self.set_notes(Some(client), None),
            Event::SetNotes(notes) => self.set_notes(None, Some(notes)),
            Event::SetTechnician(technician) => {
                if technician == self.settings.technician {
                    return vec![];
                }
                self.settings.technician = technician;
                vec![self.save_settings()]
            }
            Event::SetLogo(png) => {
                if png.is_empty() {
                    return vec![];
                }
                self.has_logo = true;
                self.logo_problem = None;
                vec![Effect::SaveLogo(png)]
            }
            Event::ChooseLogo => {
                self.logo_problem = None;
                vec![Effect::ChooseLogo]
            }
            Event::LogoRefused(key) => {
                self.logo_problem = Some(key);
                vec![]
            }
            Event::CopyComparison => {
                if self.compared().is_none() {
                    return vec![];
                }
                self.export = None;
                vec![Effect::CopyComparison]
            }
            Event::RemoveLogo => {
                self.has_logo = false;
                vec![Effect::ClearLogo]
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
                    if let Some(check) = self.current.as_ref().map(Checked::saved) {
                        effects.push(self.remember(check));
                    }
                } else {
                    // Every saved check goes at once; this run's receipt stays on screen.
                    self.checks.clear();
                    self.opened = None;
                    self.drop_stale_compare();
                    effects.push(Effect::ClearChecks);
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
                self.settings.updates_chosen = true;
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
                self.clear_for_a_new_check();
                let mut effects = self.update(Event::Finished(Ok(report), seconds));
                // The print preview open meanwhile shows the quick receipt: Print would print the new one.
                if self.panel == Panel::Preview {
                    effects.push(Effect::RenderPreview);
                }
                effects
            }
            Event::DeepFinished(Err(error), _) => self.update(Event::DeepRefused(error)),
            Event::DeepRefused(error) => {
                self.deep = Deep::Problem(match error {
                    DeepError::Cancelled => "deep.cancelled",
                    DeepError::Failed => "deep.failed",
                    DeepError::Folder => "deep.folder",
                });
                vec![]
            }
            // Print shows the pages first; printing happens from the preview.
            Event::PrintAsked => {
                if self.shown_report().is_none() {
                    return vec![];
                }
                self.panel = Panel::Preview;
                vec![Effect::RenderPreview]
            }
            Event::PrintConfirmed => {
                if self.panel != Panel::Preview || self.shown_report().is_none() {
                    return vec![];
                }
                self.panel = Panel::Main;
                self.export = None;
                vec![Effect::Print]
            }
            Event::SaveAsked | Event::CopyAsked => {
                if self.shown_report().is_none() {
                    return vec![];
                }
                self.export = None;
                vec![if event == Event::SaveAsked { Effect::Save } else { Effect::Copy }]
            }
            Event::CreateLink { expires, password } => {
                if self.share == ShareStatus::Sending || self.shown_report().is_none() {
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
                        let link = SavedLink { id: shared.id.clone(), url: shared.url.clone(), delete_token: shared.delete_token.clone(), expires_at: shared.expires_at.clone() };
                        self.links.push(link.clone());
                        self.share = ShareStatus::Done(shared);
                        vec![Effect::AddLink(link)]
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
                        self.links_status = Some(key);
                        vec![Effect::ForgetLinks(vec![id])]
                    }
                    Err(()) => {
                        self.links_status = Some("links.delete_failed");
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
                        let forget: Vec<String> = self.links.iter().filter(|l| gone.contains(&l.id)).map(|l| l.id.clone()).collect();
                        self.links.retain(|l| !gone.contains(&l.id));
                        self.links_gone = before - self.links.len();
                        self.links_status = Some(match self.links_gone {
                            0 => "links.all_work",
                            1 => "links.one_gone",
                            _ => "links.some_gone",
                        });
                        if forget.is_empty() { vec![] } else { vec![Effect::ForgetLinks(forget)] }
                    }
                    Err(()) => {
                        self.links_status = Some("links.check_failed");
                        vec![]
                    }
                }
            }
            Event::LinksLoaded(links) => {
                self.links = links;
                vec![]
            }
            Event::Exported(key) => {
                self.export = key;
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
            // The try is recorded either way (update::answered), so a failure waits before the next.
            Event::UpdateChecked(record) => {
                self.update = record;
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
        assert_eq!(s.update(Event::Finished(Ok(r.clone()), 0.7)), vec![Effect::AddCheck(Box::new(saved(&r)))]);
        assert_eq!((s.main_view(), s.mat()), (MainView::Result, Mat::Live));
        assert_eq!(s.last_check(), Some(&r));
    }

    #[test]
    fn nothing_is_remembered_when_remember_is_off() {
        let mut s = State::default();
        s.update(Event::Loaded { settings: Settings { remember: false, ..Settings::default() }, checks: vec![], links: vec![], has_logo: false });
        s.update(Event::Start);
        assert_eq!(s.update(Event::Finished(Ok(report("2026-10-09T10:00:00Z")), 0.7)), vec![]);
        assert!(s.checks.is_empty());
    }

    #[test]
    fn a_failed_first_check_goes_back_home_and_says_so() {
        let mut s = State::default();
        s.update(Event::Loaded { settings: Settings::default(), checks: vec![saved(&report("2026-10-08T10:00:00Z"))], links: vec![], has_logo: false });
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
        s.update(Event::Loaded { settings: Settings { remember: false, ..Settings::default() }, checks: vec![], links: vec![], has_logo: false });
        s.update(Event::Start);
        s.update(Event::Finished(Ok(report("2026-10-09T10:00:00Z")), 0.7));
        s.update(Event::Start);
        s.update(Event::Finished(Err(ScanProblem::Failed), 1.0));
        assert_eq!((s.main_view(), s.mat()), (MainView::Result, Mat::Live));
        assert_eq!(s.problem, Some(ScanProblem::Failed));
        assert_eq!(s.update(Event::SaveAsked), vec![Effect::Save]);
        // The next check clears the problem.
        s.update(Event::Start);
        assert_eq!(s.problem, None);
    }

    #[test]
    fn coming_back_shows_the_last_receipt_faded() {
        let mut s = State::default();
        s.update(Event::Loaded { settings: Settings::default(), checks: vec![saved(&report("2026-10-08T10:00:00Z")), saved(&report("2026-10-01T10:00:00Z"))], links: vec![], has_logo: false });
        assert_eq!((s.main_view(), s.mat()), (MainView::Home, Mat::Last));
        assert_eq!(s.last_check().map(|r| r.collected_at.as_str()), Some("2026-10-08T10:00:00Z"), "the newest");
        assert_eq!(s.shown_report(), None, "the faded one isn't what Save would save");
    }

    #[test]
    fn turning_remember_off_forgets_every_check_at_once() {
        let mut s = with_checks(&["2026-10-01T10:00:00Z"]);
        s.update(Event::OpenCheck("2026-10-01T10:00:00Z".into()));
        let effects = s.update(Event::SetRemember(false));
        assert!(effects.contains(&Effect::ClearChecks), "{effects:?}");
        assert!(effects.iter().any(|e| matches!(e, Effect::SaveSettings(st) if !st.remember)));
        assert!(s.checks.is_empty());
        assert_eq!((s.opened.clone(), s.mat()), (None, Mat::Live), "back to this run's receipt");
        // While off, nothing is added, and loading another window's list keeps it empty.
        s.update(Event::Start);
        assert!(!s.update(Event::Finished(Ok(report("2026-10-09T12:00:00Z")), 0.7)).iter().any(|e| matches!(e, Effect::AddCheck(_))));
        s.update(Event::ChecksLoaded(vec![saved(&report("2026-10-01T10:00:00Z"))]));
        assert!(s.checks.is_empty());
    }

    #[test]
    fn turning_remember_on_keeps_this_check_with_its_client_and_notes() {
        let mut s = State::default();
        s.update(Event::Loaded { settings: Settings { remember: false, ..Settings::default() }, checks: vec![], links: vec![], has_logo: false });
        s.update(Event::Start);
        let r = report("2026-10-09T10:00:00Z");
        s.update(Event::Finished(Ok(r.clone()), 0.7));
        assert_eq!(s.update(Event::SetClient("Mario Rossi".into())), vec![], "in memory only while off");
        assert_eq!(s.shown_notes(), ("Mario Rossi", ""));
        let kept = SavedCheck { client: "Mario Rossi".into(), ..saved(&r) };
        assert!(s.update(Event::SetRemember(true)).contains(&Effect::AddCheck(Box::new(kept))));
        assert_eq!(s.checks.len(), 1);
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
        s.update(Event::CreateLink { expires: Expiry::Week, password: None });
        s.update(Event::Open(Panel::Settings));
        s.update(Event::Back);
        assert_eq!(s.panel, Panel::Share);
        s.update(Event::LinkCreated(Ok(shared("abc"))));
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
        // Trying again can't help when the folder itself won't do: it says so.
        s.update(Event::DeepAsked);
        s.update(Event::DeepRefused(DeepError::Folder));
        assert_eq!(s.deep, Deep::Problem("deep.folder"));
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
    fn turning_the_update_check_on_or_off_is_a_choice_that_gets_saved() {
        let mut s = State::default();
        assert!(!s.settings.updates_chosen);
        s.update(Event::SetUpdates(false));
        assert!(s.settings.updates_chosen);
    }

    #[test]
    fn with_updates_off_nothing_is_asked() {
        let mut s = State::default();
        s.update(Event::Loaded { settings: Settings { updates: false, ..Settings::default() }, checks: vec![], links: vec![], has_logo: false });
        assert_eq!(s.update(Event::UpdateRecord(UpdateRecord::default())), vec![]);
    }

    #[test]
    fn a_newer_version_shows_a_note_until_updates_are_turned_off() {
        let mut s = State::default();
        let record = UpdateRecord { checked_at: 1_000, tried_at: 1_000, failures: 0, latest: Some(latest("999.0.0")) };
        let found = s.update(Event::UpdateChecked(record.clone()));
        assert_eq!(found, vec![Effect::SaveUpdate(record)]);
        assert_eq!(s.update_note().map(|l| l.version.as_str()), Some("999.0.0"));
        assert_eq!(s.update(Event::OpenUpdate), vec![Effect::OpenUpdatePage]);
        s.update(Event::SetUpdates(false));
        assert_eq!(s.update_note(), None);
        assert_eq!(s.update(Event::OpenUpdate), vec![]);
    }

    #[test]
    fn the_same_version_or_a_failed_check_says_nothing() {
        let mut s = State::default();
        s.update(Event::UpdateChecked(UpdateRecord { checked_at: 1_000, tried_at: 1_000, failures: 0, latest: Some(latest(crate::update::CURRENT)) }));
        assert_eq!(s.update_note(), None);
        let failed = UpdateRecord { checked_at: 1_000, tried_at: 2_000, failures: 1, latest: Some(latest(crate::update::CURRENT)) };
        assert_eq!(s.update(Event::UpdateChecked(failed.clone())), vec![Effect::SaveUpdate(failed)], "a failed check is recorded, so it waits before asking again");
        assert_eq!(s.update_note(), None);
    }

    #[test]
    fn a_finished_deep_check_clears_what_the_last_one_left_on_screen() {
        // As a new check does: an old problem, an export message and the private card go.
        let mut s = checked_state();
        s.update(Event::Start);
        s.update(Event::Finished(Err(ScanProblem::Failed), 1.0));
        s.update(Event::Exported(Some("export.saved")));
        s.private_shown = true;
        s.update(Event::DeepAsked);
        let mut deep = report("2026-10-09T11:00:00Z");
        deep.elevated = true;
        s.update(Event::DeepFinished(Ok(deep), 1.2));
        assert_eq!((s.problem, s.export, s.private_shown), (None, None, false));
        assert!(s.done_sources.is_empty());
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
        assert!(effects.contains(&Effect::AddCheck(Box::new(saved(&deep)))), "{effects:?}");
        // A refused or failed one keeps the quick receipt and says why.
        s.update(Event::DeepAsked);
        s.update(Event::DeepFinished(Err(DeepError::Cancelled), 0.0));
        assert_eq!(s.deep, Deep::Problem("deep.cancelled"));
    }

    #[test]
    fn a_deep_check_finishing_under_the_print_preview_redraws_it() {
        // The preview must show what Print will print: the Deep receipt, once it's in.
        let mut s = checked_state();
        s.update(Event::DeepAsked);
        s.update(Event::PrintAsked);
        let mut deep = report("2026-10-09T11:00:00Z");
        deep.elevated = true;
        let effects = s.update(Event::DeepFinished(Ok(deep), 1.4));
        assert_eq!(s.panel, Panel::Preview);
        assert!(effects.contains(&Effect::RenderPreview), "{effects:?}");
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
        assert!(matches!(effects.as_slice(), [Effect::AddLink(l)] if l.id == "abc" && l.delete_token == "t"), "{effects:?}");
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
        assert_eq!(s.update(Event::LinkDeleted(Ok(("a".into(), crate::share::Deleted::Now)))), vec![Effect::ForgetLinks(vec!["a".into()])]);
        assert_eq!(s.links.iter().map(|l| l.id.as_str()).collect::<Vec<_>>(), vec!["b"]);
        assert_eq!(s.links_status, Some("links.deleted"));
        s.update(Event::DeleteLink("b".into()));
        s.update(Event::LinkDeleted(Err(())));
        assert_eq!(s.links.len(), 1, "kept: it may still be public");
        assert_eq!(s.links_status, Some("links.delete_failed"));
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
        assert_eq!(done, vec![Effect::ForgetLinks(vec!["a".into(), "c".into()])]);
        assert_eq!((s.links_status, s.links_gone), (Some("links.some_gone"), 2));
        s.update(Event::CheckLinks);
        assert_eq!(s.update(Event::LinksChecked(Ok(vec![]))), vec![], "nothing gone, nothing to save");
        assert_eq!(s.links_status, Some("links.all_work"));
        s.update(Event::CheckLinks);
        s.update(Event::LinksChecked(Ok(vec!["b".into()])));
        assert_eq!(s.links_status, Some("links.one_gone"));
        assert_eq!(State::default().update(Event::CheckLinks), vec![], "nothing to check");
    }

    #[test]
    fn opening_my_links_reads_them_again_so_another_windows_links_show() {
        let mut s = with_links(&["a"]);
        s.update(Event::Back);
        assert_eq!(s.update(Event::Open(Panel::Links)), vec![Effect::ReloadLinks]);
        let mut both = s.links.clone();
        both.push(SavedLink { id: "b".into(), url: "https://fettle.page/r/b".into(), delete_token: "token-b".into(), expires_at: "2026-11-01T00:00:00Z".into() });
        s.update(Event::LinksLoaded(both));
        assert_eq!(s.links.iter().map(|l| l.id.as_str()).collect::<Vec<_>>(), vec!["a", "b"]);
    }

    #[test]
    fn a_check_that_cant_reach_the_server_keeps_every_link() {
        let mut s = with_links(&["a"]);
        s.update(Event::CheckLinks);
        assert_eq!(s.update(Event::LinksChecked(Err(()))), vec![]);
        assert_eq!(s.links.len(), 1);
        assert_eq!(s.links_status, Some("links.check_failed"));
    }

    #[test]
    fn deleting_a_link_already_deleted_on_the_website_says_so() {
        let mut s = with_links(&["a"]);
        s.update(Event::DeleteLink("a".into()));
        s.update(Event::LinkDeleted(Ok(("a".into(), crate::share::Deleted::AlreadyGone))));
        assert!(s.links.is_empty());
        assert_eq!(s.links_status, Some("links.already_gone"));
    }

    #[test]
    fn an_export_message_lasts_until_the_next_check() {
        let mut s = checked_state();
        s.update(Event::Exported(Some("export.saved")));
        assert_eq!(s.export, Some("export.saved"));
        s.update(Event::Start);
        assert_eq!(s.export, None);
    }

    fn saved(r: &Report) -> SavedCheck {
        SavedCheck { report: r.clone(), client: String::new(), notes: String::new() }
    }

    /// These checks saved from before, then this run's check (2026-10-09 at 10:00).
    fn with_checks(ats: &[&str]) -> State {
        let mut s = State::default();
        s.update(Event::Loaded { settings: Settings::default(), checks: ats.iter().map(|at| saved(&report(at))).collect(), links: vec![], has_logo: false });
        s.update(Event::Start);
        s.update(Event::Finished(Ok(report("2026-10-09T10:00:00Z")), 0.7));
        s
    }

    fn ids(s: &State) -> Vec<&str> {
        s.checks.iter().map(|c| c.id()).collect()
    }

    #[test]
    fn only_the_last_ten_checks_are_kept_newest_first() {
        let mut s = State::default();
        for hour in 10..22 {
            s.update(Event::Start);
            s.update(Event::Finished(Ok(report(&format!("2026-10-09T{hour}:00:00Z"))), 0.7));
        }
        assert_eq!(s.checks.len(), 10);
        assert_eq!(s.checks[0].id(), "2026-10-09T21:00:00Z");
        // A Deep check of the same moment (or the same check saved twice) is never listed twice.
        s.update(Event::SetRemember(false));
        s.update(Event::SetRemember(true));
        s.update(Event::SetRemember(true));
        assert_eq!(ids(&s), ["2026-10-09T21:00:00Z"]);
    }

    #[test]
    fn opening_my_checks_reads_them_again_so_another_windows_checks_show() {
        let mut s = with_checks(&[]);
        assert_eq!(s.update(Event::Open(Panel::Checks)), vec![Effect::ReloadChecks]);
        assert_eq!(s.panel, Panel::Checks);
        s.update(Event::ChecksLoaded(vec![saved(&report("2026-10-09T10:00:00Z")), saved(&report("2026-10-09T09:00:00Z"))]));
        assert_eq!(ids(&s), ["2026-10-09T10:00:00Z", "2026-10-09T09:00:00Z"]);
    }

    #[test]
    fn an_opened_past_check_lies_faded_on_the_mat_and_the_exports_act_on_it() {
        let old = report("2026-10-01T10:00:00Z");
        let mut s = with_checks(&["2026-10-01T10:00:00Z"]);
        s.update(Event::Open(Panel::Checks));
        assert_eq!(s.update(Event::OpenCheck("2026-10-01T10:00:00Z".into())), vec![]);
        assert_eq!((s.panel, s.main_view(), s.mat()), (Panel::Main, MainView::Past, Mat::Past));
        assert_eq!(s.shown_report(), Some(&old));
        assert_eq!(s.update(Event::SaveAsked), vec![Effect::Save]);
        assert_eq!(s.update(Event::CopyAsked), vec![Effect::Copy]);
        assert_eq!(s.update(Event::PrintAsked), vec![Effect::RenderPreview]);
        s.update(Event::Back);
        s.update(Event::Open(Panel::Share));
        assert_eq!(s.panel, Panel::Share);
        s.update(Event::Back);
        // Closing it brings back this run's receipt; so does a new check.
        s.update(Event::CloseCheck);
        assert_eq!((s.main_view(), s.mat()), (MainView::Result, Mat::Live));
        assert_eq!(s.shown_report().map(|r| r.collected_at.as_str()), Some("2026-10-09T10:00:00Z"));
        s.update(Event::OpenCheck("2026-10-01T10:00:00Z".into()));
        s.update(Event::Start);
        assert_eq!((s.opened.clone(), s.mat()), (None, Mat::Blank));
        // Opening this run's own check is just showing it; an unknown one does nothing.
        let mut s = with_checks(&[]);
        s.update(Event::OpenCheck("2026-10-09T10:00:00Z".into()));
        s.update(Event::OpenCheck("never saved".into()));
        assert_eq!((s.opened.clone(), s.mat()), (None, Mat::Live));
    }

    #[test]
    fn a_past_check_can_be_saved_with_no_check_run_this_time() {
        let mut s = State::default();
        s.update(Event::Loaded { settings: Settings::default(), checks: vec![saved(&report("2026-10-01T10:00:00Z"))], links: vec![], has_logo: false });
        assert_eq!(s.update(Event::SaveAsked), vec![], "the faded last check isn't opened yet");
        s.update(Event::OpenCheck("2026-10-01T10:00:00Z".into()));
        assert_eq!(s.update(Event::SaveAsked), vec![Effect::Save]);
        assert!(matches!(s.update(Event::CreateLink { expires: Expiry::Week, password: None }).as_slice(), [Effect::Share { .. }]));
    }

    #[test]
    fn a_check_is_deleted_from_the_list_and_the_mat() {
        let mut s = with_checks(&["2026-10-01T10:00:00Z", "2026-10-02T10:00:00Z"]);
        s.update(Event::OpenCheck("2026-10-01T10:00:00Z".into()));
        assert_eq!(s.update(Event::DeleteCheck("2026-10-01T10:00:00Z".into())), vec![Effect::DeleteCheck("2026-10-01T10:00:00Z".into())]);
        assert_eq!(ids(&s), ["2026-10-09T10:00:00Z", "2026-10-02T10:00:00Z"]);
        assert_eq!((s.opened.clone(), s.mat()), (None, Mat::Live));
        assert_eq!(s.update(Event::DeleteCheck("never saved".into())), vec![]);
        // Deleting this run's own entry keeps its receipt on screen; it's just no longer saved.
        s.update(Event::DeleteCheck("2026-10-09T10:00:00Z".into()));
        assert_eq!(s.mat(), Mat::Live);
    }

    #[test]
    fn after_a_check_the_previous_one_is_offered_for_comparing() {
        let mut s = checked_state();
        assert_eq!(s.previous_check(), None, "nothing earlier saved");
        assert_eq!(s.update(Event::CompareWithPrevious), vec![]);
        let mut s = with_checks(&["2026-10-01T10:00:00Z", "2026-10-05T10:00:00Z"]);
        assert_eq!(s.previous_check().map(SavedCheck::id), Some("2026-10-05T10:00:00Z"));
        s.update(Event::CompareWithPrevious);
        assert_eq!(s.compare, Some(("2026-10-05T10:00:00Z".to_string(), "2026-10-09T10:00:00Z".to_string())));
        assert_eq!(s.panel, Panel::Compare);
        let (before, after) = s.compared().unwrap();
        assert_eq!((before.collected_at.as_str(), after.collected_at.as_str()), ("2026-10-05T10:00:00Z", "2026-10-09T10:00:00Z"));
        s.update(Event::Back);
        assert_eq!((s.panel, s.compare.clone()), (Panel::Main, None));
        // With no check run this time, nothing is "the previous" one.
        let mut s = State::default();
        s.update(Event::Loaded { settings: Settings::default(), checks: vec![saved(&report("2026-10-01T10:00:00Z"))], links: vec![], has_logo: false });
        assert_eq!(s.previous_check(), None);
    }

    #[test]
    fn two_saved_checks_can_be_compared() {
        let mut s = with_checks(&["2026-10-01T10:00:00Z", "2026-10-05T10:00:00Z"]);
        s.update(Event::Open(Panel::Checks));
        s.update(Event::CompareChecks("2026-10-01T10:00:00Z".into(), "2026-10-05T10:00:00Z".into()));
        assert_eq!((s.panel, s.compare.clone()), (Panel::Compare, Some(("2026-10-01T10:00:00Z".to_string(), "2026-10-05T10:00:00Z".to_string()))));
        // Deleting one of them (in another window, say) ends the comparison.
        s.update(Event::ChecksLoaded(vec![saved(&report("2026-10-05T10:00:00Z"))]));
        assert_eq!((s.panel, s.compare.clone()), (Panel::Main, None));
        // A check with itself, or one that isn't saved, can't be compared.
        let mut s = with_checks(&["2026-10-01T10:00:00Z"]);
        s.update(Event::CompareChecks("2026-10-01T10:00:00Z".into(), "2026-10-01T10:00:00Z".into()));
        s.update(Event::CompareChecks("2026-10-01T10:00:00Z".into(), "gone".into()));
        assert_eq!(s.compare, None);
    }

    #[test]
    fn client_and_notes_go_with_the_check_shown() {
        let mut s = with_checks(&["2026-10-01T10:00:00Z"]);
        let effects = s.update(Event::SetClient("Mario Rossi".into()));
        assert_eq!(effects, vec![Effect::SaveCheckNotes { id: "2026-10-09T10:00:00Z".into(), client: "Mario Rossi".into(), notes: String::new() }]);
        s.update(Event::OpenCheck("2026-10-01T10:00:00Z".into()));
        assert_eq!(s.shown_notes(), ("", ""), "the past check has its own");
        let effects = s.update(Event::SetNotes("Fan cleaned".into()));
        assert_eq!(effects, vec![Effect::SaveCheckNotes { id: "2026-10-01T10:00:00Z".into(), client: String::new(), notes: "Fan cleaned".into() }]);
        assert_eq!((s.checks[1].notes.as_str(), s.checks[0].client.as_str()), ("Fan cleaned", "Mario Rossi"));
        s.update(Event::CloseCheck);
        assert_eq!(s.shown_notes(), ("Mario Rossi", ""));
        // A new check starts with empty fields.
        s.update(Event::Start);
        s.update(Event::Finished(Ok(report("2026-10-09T11:00:00Z")), 0.7));
        assert_eq!(s.shown_notes(), ("", ""));
        assert_eq!(State::default().update(Event::SetClient("x".into())), vec![], "no check shown");
    }

    #[test]
    fn technician_details_and_the_logo_are_saved() {
        let mut s = State::default();
        let me = Technician { business: "PC Rossi".into(), name: "Mario Rossi".into(), contact: "055 123456".into() };
        assert!(matches!(s.update(Event::SetTechnician(me.clone())).as_slice(), [Effect::SaveSettings(st)] if st.technician == me));
        assert_eq!(s.settings.technician, me);
        assert_eq!(s.update(Event::SetLogo(vec![1, 2, 3])), vec![Effect::SaveLogo(vec![1, 2, 3])]);
        assert!(s.has_logo);
        assert_eq!(s.update(Event::RemoveLogo), vec![Effect::ClearLogo]);
        assert!(!s.has_logo);
        s.update(Event::Loaded { settings: Settings::default(), checks: vec![], links: vec![], has_logo: true });
        assert!(s.has_logo);
    }

    #[test]
    fn a_field_left_unchanged_writes_nothing() {
        // Leaving a field always says so; only a change is saved.
        let mut s = with_checks(&[]);
        assert_eq!(s.update(Event::SetClient(String::new())), vec![]);
        assert_eq!(s.update(Event::SetNotes(String::new())), vec![]);
        assert_eq!(s.update(Event::SetClient("Mario Rossi".into())).len(), 1);
        assert_eq!(s.update(Event::SetClient("Mario Rossi".into())), vec![]);
        assert_eq!(s.update(Event::SetTechnician(Technician::default())), vec![]);
    }

    #[test]
    fn a_comparison_is_copied_as_text() {
        let mut s = with_checks(&["2026-10-05T10:00:00Z"]);
        assert_eq!(s.update(Event::CopyComparison), vec![], "nothing compared yet");
        s.update(Event::Exported(Some("export.saved")));
        s.update(Event::CompareWithPrevious);
        assert_eq!(s.export, None, "the receipt's last message doesn't follow into the comparison");
        assert_eq!(s.update(Event::CopyComparison), vec![Effect::CopyComparison]);
        s.update(Event::Exported(Some("export.copied")));
        assert_eq!(s.export, Some("export.copied"));
    }

    #[test]
    fn a_logo_that_cant_be_used_says_why_until_the_next_try() {
        let mut s = State::default();
        s.update(Event::Open(Panel::Settings));
        assert_eq!(s.update(Event::ChooseLogo), vec![Effect::ChooseLogo]);
        s.update(Event::LogoRefused("settings.logo.e.type"));
        assert_eq!((s.logo_problem, s.has_logo), (Some("settings.logo.e.type"), false));
        s.update(Event::ChooseLogo);
        assert_eq!(s.logo_problem, None, "a new try starts clean");
        s.update(Event::LogoRefused("settings.logo.e.too_big"));
        s.update(Event::SetLogo(vec![1]));
        assert_eq!((s.logo_problem, s.has_logo), (None, true));
        s.update(Event::LogoRefused("settings.logo.e.too_big"));
        s.update(Event::Back);
        s.update(Event::Open(Panel::Settings));
        assert_eq!(s.logo_problem, None, "gone when Settings opens again");
    }
}
