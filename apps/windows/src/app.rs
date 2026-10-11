//! The app: its window, its state, and the services it uses (each replaceable by a fake in tests).
//! Every change is an `Event`; `pump` applies queued events to the state, runs the effects they
//! ask for and puts the result on the window. Background work posts its events into the same queue.

use crate::state::{Effect, Event, MainView, Panel, PrivateCard, PrivateKind, ScanProblem, State, ThemeChoice};
use crate::store::{FileStore, Store};
use crate::{AppWindow, Copy, copy, view};
use fettle_facts::Report;
use fettle_verdicts::Locale;
use slint::ComponentHandle;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Reads this PC. The real one is fettle-collect; reports each source as it settles.
pub trait Scanner: Send + Sync {
    fn scan(&self, on_source: &dyn Fn(&str)) -> Report;
}

/// Work done away from the window (the scan, uploads). The real one is a thread.
pub type Job = Box<dyn FnOnce() + Send>;
pub trait Background {
    fn spawn(&self, job: Job);
}

/// Windows' save dialog, owned by the app's window. None when cancelled.
pub trait Dialogs {
    fn save(&self, owner: &slint::Window, suggested: &str, filters: &[(String, Vec<String>)]) -> Option<std::path::PathBuf>;
}

pub trait Clipboard {
    fn set_text(&self, text: &str) -> bool;
}

/// Windows' print dialog, then the receipt on paper. Ok(false) when cancelled.
pub trait Printer {
    /// Whether Windows can print here at all (its print service runs and a printer is installed).
    /// When it can't, Print saves a PDF instead: making a PDF needs nothing from Windows.
    fn available(&self) -> bool {
        true
    }
    fn print(&self, owner: &slint::Window, model: &fettle_receipt::ReceiptModel) -> Result<bool, String>;
}

/// The Deep check: Windows' prompt (owned by this window handle), then the elevated scan and how
/// long it ran.
pub type DeepScan = Arc<dyn Fn(isize) -> Result<(Report, f64), crate::state::DeepError> + Send + Sync>;

/// What the app needs from the outside world. Real ones by default; tests swap in fakes.
pub struct Deps {
    /// Windows' display language, for "Auto".
    pub system_lang: Locale,
    pub scanner: Arc<dyn Scanner>,
    pub store: Rc<dyn Store>,
    pub background: Rc<dyn Background>,
    /// Asks the window's thread to `pump` (events posted from another thread are waiting).
    pub wake: Arc<dyn Fn() + Send + Sync>,
    /// The viewer's offset from UTC, in minutes, at an instant (ms since 1970).
    pub zone: Arc<dyn Fn(i64) -> i32 + Send + Sync>,
    pub now_ms: Arc<dyn Fn() -> i64 + Send + Sync>,
    /// Reads this PC's name and serial numbers, for the privacy switch only.
    pub private: Arc<dyn Fn() -> PrivateCard + Send + Sync>,
    pub dialogs: Rc<dyn Dialogs>,
    pub clipboard: Rc<dyn Clipboard>,
    pub printer: Rc<dyn Printer>,
    /// The paper the print preview lays out on: Windows' own (Letter where it measures in inches).
    pub paper: Rc<dyn Fn() -> crate::print::layout::PageSize>,
    /// HTTPS to the share API (Windows' TLS); tests use a fake server.
    pub http: Arc<dyn crate::share::Http>,
    pub share_origin: String,
    /// Opens a link in the default browser.
    pub open_url: Rc<dyn Fn(&str) -> bool>,
    /// Whether Windows' apps are set to dark, for the Auto theme (None: Windows doesn't say). Read at
    /// launch and every few seconds, so Auto follows a change made while Fettle is open.
    pub system_dark: Rc<dyn Fn() -> Option<bool>>,
    /// The Deep check's elevated scan: Windows asks first (in a prompt owned by the window, its HWND,
    /// 0 for none), then the helper's report and how long its scan took (not counting the prompt).
    /// Runs off the window's thread; Err says why there's none.
    pub deep_scan: DeepScan,
}

thread_local! {
    // The app on this thread, for events posted from other threads.
    static APP: RefCell<Weak<Inner>> = const { RefCell::new(Weak::new()) };
}

impl Default for Deps {
    fn default() -> Self {
        Deps {
            system_lang: crate::win::system_lang(),
            scanner: Arc::new(crate::scan::RealScanner),
            #[cfg(windows)]
            store: Rc::new(FileStore::new(FileStore::default_dir()).with_machine_updates(crate::win::machine_updates())),
            #[cfg(not(windows))]
            store: Rc::new(FileStore::new(FileStore::default_dir())),
            background: Rc::new(crate::scan::Threads),
            wake: Arc::new(|| {
                let _ = slint::invoke_from_event_loop(|| {
                    if let Some(inner) = APP.with(|a| a.borrow().upgrade()) {
                        Fettle(inner).pump();
                    }
                });
            }),
            zone: Arc::new(crate::zone::local_offset_minutes),
            now_ms: Arc::new(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)),
            private: Arc::new(read_private),
            dialogs: Rc::new(crate::exports::RealDialogs),
            clipboard: Rc::new(crate::exports::RealClipboard),
            printer: Rc::new(crate::exports::RealPrinter),
            paper: Rc::new(crate::exports::paper_size),
            http: Arc::new(crate::share::RealHttp::default()),
            share_origin: crate::share::origin(),
            open_url: Rc::new(crate::exports::open_in_browser),
            #[cfg(windows)]
            deep_scan: Arc::new(crate::elevate::deep_scan),
            #[cfg(not(windows))]
            deep_scan: Arc::new(|_| Err(crate::state::DeepError::Failed)),
            #[cfg(windows)]
            system_dark: Rc::new(crate::win::system_dark),
            #[cfg(not(windows))]
            system_dark: Rc::new(|| None),
        }
    }
}

/// The privacy switch's details, from fettle-collect (Windows only).
fn read_private() -> PrivateCard {
    #[cfg(windows)]
    {
        use fettle_collect::private::PrivateKind as K;
        let d = fettle_collect::private::read();
        PrivateCard {
            pc_name: d.pc_name,
            items: d
                .items
                .into_iter()
                .map(|i| {
                    let kind = match i.kind {
                        K::System => PrivateKind::System,
                        K::Board => PrivateKind::Board,
                        K::Disk => PrivateKind::Disk(i.label),
                    };
                    (kind, i.serial)
                })
                .collect(),
        }
    }
    #[cfg(not(windows))]
    PrivateCard::default()
}

struct Inner {
    window: AppWindow,
    deps: Deps,
    state: RefCell<State>,
    queue: Arc<Mutex<VecDeque<Event>>>,
    pumping: RefCell<bool>,
    /// The privacy switch's card, in memory only, while the switch is on.
    private: RefCell<Option<PrivateCard>>,
    view_cache: RefCell<view::Cache>,
    /// The print preview's pages, while it's open.
    preview: RefCell<Option<view::Preview>>,
    /// An update check is on its way (launch and the switch can both ask).
    checking: std::cell::Cell<bool>,
    /// Every ten minutes: a window left open for days still checks as often as it's set to.
    _ticker: slint::Timer,
    /// Every few seconds: Windows' light or dark setting, for Auto.
    _scheme: slint::Timer,
}

pub struct Fettle(Rc<Inner>);

impl Fettle {
    pub fn new(deps: Deps) -> Result<Fettle, slint::PlatformError> {
        let window = AppWindow::new()?;
        window.global::<Copy>().on_tr(|key, lang| copy::any(&key, Locale::from_tag(&lang)).into());
        let loaded = Event::Loaded { settings: deps.store.load_settings(), last: deps.store.load_last(), links: deps.store.load_links((deps.now_ms)()) };
        let inner = Rc::new(Inner { window, deps, state: RefCell::new(State::default()), queue: Arc::default(), pumping: RefCell::new(false), private: RefCell::new(None), view_cache: RefCell::default(), preview: RefCell::new(None), checking: Default::default(), _ticker: slint::Timer::default(), _scheme: slint::Timer::default() });
        APP.with(|a| *a.borrow_mut() = Rc::downgrade(&inner));
        let app = Fettle(inner);
        app.wire();
        app.dispatch(loaded);
        app.dispatch(Event::UpdateRecord(app.0.deps.store.load_update()));
        app.follow_windows_scheme();
        let weak = Rc::downgrade(&app.0);
        app.0._scheme.start(slint::TimerMode::Repeated, std::time::Duration::from_secs(3), move || {
            if let Some(inner) = weak.upgrade() {
                Fettle(inner).follow_windows_scheme();
            }
        });
        let weak = Rc::downgrade(&app.0);
        app.0._ticker.start(slint::TimerMode::Repeated, std::time::Duration::from_secs(600), move || {
            if let Some(inner) = weak.upgrade() {
                Fettle(inner).dispatch(Event::UpdateTick);
            }
        });
        Ok(app)
    }

    /// Windows' light or dark setting, for the Auto theme (unchanged when Windows doesn't say).
    fn follow_windows_scheme(&self) {
        if let Some(dark) = (self.0.deps.system_dark)() {
            let theme = self.0.window.global::<crate::Theme>();
            if theme.get_system_dark() != dark {
                theme.set_system_dark(dark);
            }
        }
    }

    pub fn window(&self) -> &AppWindow {
        &self.0.window
    }

    /// Queues an event and applies everything queued.
    pub fn dispatch(&self, event: Event) {
        self.0.queue.lock().expect("event queue").push_back(event);
        self.pump();
    }

    /// Applies the queued events (also those posted by background work), runs their effects and
    /// redraws. Re-entrant calls just leave their events for the running pump.
    pub fn pump(&self) {
        if self.0.pumping.replace(true) {
            return;
        }
        loop {
            let Some(event) = self.0.queue.lock().expect("event queue").pop_front() else { break };
            let before = self.0.state.borrow().main_view();
            if matches!(event, Event::UpdateChecked(..)) {
                self.0.checking.set(false);
            }
            if let Event::PrivateLoaded(card) = &event {
                // Kept only while the switch is still on (it may have gone off meanwhile).
                if self.0.state.borrow().settings.show_private {
                    *self.0.private.borrow_mut() = Some(card.clone());
                }
            }
            let effects = self.0.state.borrow_mut().update(event);
            for effect in effects {
                self.run(effect);
            }
            if before != MainView::Result && self.0.state.borrow().main_view() == MainView::Result {
                view::feed(&self.0.window);
            }
        }
        self.0.pumping.replace(false);
        // The preview's pages go as soon as it closes.
        if self.0.state.borrow().panel != Panel::Preview {
            self.0.preview.borrow_mut().take();
        }
        let private = self.0.private.borrow();
        let preview = self.0.preview.borrow();
        let ctx = view::Ctx { system_lang: self.0.deps.system_lang, zone: &*self.0.deps.zone, now_ms: (self.0.deps.now_ms)(), private: private.as_ref(), preview: preview.as_ref() };
        view::apply(&self.0.window, &self.0.state.borrow(), &ctx, &mut self.0.view_cache.borrow_mut());
    }

    /// A sender for events from other threads: queued, then the window's thread is woken.
    fn poster(&self) -> Arc<dyn Fn(Event) + Send + Sync> {
        let queue = self.0.queue.clone();
        let wake = self.0.deps.wake.clone();
        Arc::new(move |event| {
            queue.lock().expect("event queue").push_back(event);
            wake();
        })
    }

    fn run(&self, effect: Effect) {
        let store = &self.0.deps.store;
        match effect {
            Effect::RunScan => {
                let scanner = self.0.deps.scanner.clone();
                let post = self.poster();
                self.0.deps.background.spawn(Box::new(move || {
                    let start = Instant::now();
                    // Each source already turns its own panics into results; this catches the rest.
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scanner.scan(&|name| post(Event::SourceDone(name.to_string())))));
                    post(Event::Finished(result.map_err(|_| ScanProblem::Failed), start.elapsed().as_secs_f64()));
                }));
            }
            Effect::SaveLast(report) => store.save_last(&report),
            Effect::ClearLast => store.clear_last(),
            Effect::SaveSettings(settings) => store.save_settings(&settings),
            Effect::LoadPrivate => {
                let read = self.0.deps.private.clone();
                let post = self.poster();
                // Details that can't be read show as none, rather than a card that never appears.
                self.0.deps.background.spawn(Box::new(move || post(Event::PrivateLoaded(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| read())).unwrap_or_default()))));
            }
            Effect::ForgetPrivate => *self.0.private.borrow_mut() = None,
            Effect::Save => {
                let outcome = self.with_receipt(|report, model| crate::exports::save(&*self.0.deps.dialogs, self.0.window.window(), report, model, self.lang()));
                self.post_now(Event::Exported(outcome.flatten()));
            }
            Effect::Copy => {
                let copied = self.with_receipt(|_, model| self.0.deps.clipboard.set_text(&fettle_receipt::markdown(model)));
                self.post_now(Event::Exported(Some(if copied == Some(true) { "export.copied" } else { "export.failed" })));
            }
            Effect::Share { expires, password } => {
                let Some(report) = self.0.state.borrow().current.as_ref().map(|c| c.report.clone()) else { return };
                // What leaves this PC: the redacted report, plus the engine version so the shared
                // page shows the same verdicts.
                let report = Report { engine_version: Some(fettle_verdicts::ENGINE_VERSION.to_string()), ..report };
                if password.as_ref().is_some_and(|p| p.0.encode_utf16().count() < crate::share::PASSWORD_MIN) {
                    self.post_now(Event::LinkCreated(Err(("share.e.password_too_short", None))));
                    return;
                }
                let (http, origin, post) = (self.0.deps.http.clone(), self.0.deps.share_origin.clone(), self.poster());
                self.0.deps.background.spawn(Box::new(move || {
                    // A transport that panics still ends with a message the panel can show.
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::share::share_report(&*http, &origin, &report, Some(expires), password.as_ref().map(|p| p.0.as_str()))))
                        .unwrap_or(Err(crate::share::Failure { error: "bad_response", retry_after: None, detail: None }));
                    post(Event::LinkCreated(result.map_err(|f| crate::share::problem(&f))));
                }));
            }
            Effect::AddLink(link) => store.add_link(&link, (self.0.deps.now_ms)()),
            Effect::ForgetLinks(ids) => store.forget_links(&ids, (self.0.deps.now_ms)()),
            Effect::ReloadLinks => self.post_now(Event::LinksLoaded(store.load_links((self.0.deps.now_ms)()))),
            Effect::CopyText(text) => {
                let ok = self.0.deps.clipboard.set_text(&text);
                self.post_now(Event::LinkCopied(ok));
            }
            Effect::OpenUrl(url) => {
                (self.0.deps.open_url)(&url);
            }
            Effect::Delete(link) => {
                let (http, origin, post) = (self.0.deps.http.clone(), self.0.deps.share_origin.clone(), self.poster());
                self.0.deps.background.spawn(Box::new(move || {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::share::delete_shared(&*http, &origin, &link.id, &link.delete_token))).unwrap_or(Err(crate::share::Failure { error: "bad_response", retry_after: None, detail: None }));
                    post(Event::LinkDeleted(result.map(|how| (link.id.clone(), how)).map_err(|_| ())));
                }));
            }
            Effect::CheckLinks(links) => {
                let (http, origin, post) = (self.0.deps.http.clone(), self.0.deps.share_origin.clone(), self.poster());
                self.0.deps.background.spawn(Box::new(move || {
                    // One GET per link; any link that can't be asked about makes the whole check fail,
                    // so a lost connection never looks like "deleted".
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let mut gone = Vec::new();
                        for link in &links {
                            if !crate::share::link_exists(&*http, &origin, &link.id).map_err(|_| ())? {
                                gone.push(link.id.clone());
                            }
                        }
                        Ok(gone)
                    }))
                    .unwrap_or(Err(()));
                    post(Event::LinksChecked(result));
                }));
            }
            Effect::Print if !self.0.deps.printer.available() => {
                let saved = self.with_receipt(|report, model| crate::exports::save_pdf(&*self.0.deps.dialogs, self.0.window.window(), report, model, self.lang()));
                self.post_now(Event::Exported(Some(match saved.flatten() {
                    Some("export.saved") => "export.print_off_saved",
                    None => "export.print_off",
                    Some(_) => "export.failed",
                })));
            }
            Effect::Print => {
                let printed = self.with_receipt(|_, model| self.0.deps.printer.print(self.0.window.window(), model));
                // Printing says nothing when it works (the printer does); a failure says so.
                self.post_now(Event::Exported(match printed {
                    Some(Ok(_)) => None,
                    _ => Some("export.failed"),
                }));
            }
            Effect::DeepCheck => {
                // Windows' prompt and the helper wait on another thread, so this window keeps drawing.
                #[cfg(windows)]
                let owner = crate::win::hwnd(self.0.window.window()).map(|h| h.0 as isize).unwrap_or(0);
                #[cfg(not(windows))]
                let owner = 0;
                let (deep_scan, scanner, post) = (self.0.deps.deep_scan.clone(), self.0.deps.scanner.clone(), self.poster());
                self.0.deps.background.spawn(Box::new(move || {
                    // The time shown is the two scans', not the time spent answering Windows' prompt.
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let (elevated, elevated_secs) = deep_scan(owner)?;
                        // The user's own scan, for what belongs to them (their startup apps).
                        let start = Instant::now();
                        let user = scanner.scan(&|_| {});
                        #[allow(unused_mut)]
                        let mut deep = crate::elevate::merge_deep(user, elevated);
                        // The elevated part was cleaned with the administrator's names: clean it with the user's too.
                        #[cfg(windows)]
                        fettle_collect::redact_for_this_user(&mut deep);
                        Ok((deep, elevated_secs + start.elapsed().as_secs_f64()))
                    }))
                    .unwrap_or(Err(crate::state::DeepError::Failed));
                    match result {
                        Ok((report, secs)) => post(Event::DeepFinished(Ok(report), secs)),
                        Err(e) => post(Event::DeepFinished(Err(e), 0.0)),
                    }
                }));
            }
            Effect::RenderPreview => {
                let paper = (self.0.deps.paper)();
                let scale = self.0.window.window().scale_factor();
                let pages = self.with_receipt(|_, model| crate::exports::preview_pages(model, paper, scale)).unwrap_or_default();
                let generation = self.0.preview.borrow().as_ref().map_or(0, |p| p.generation) + 1;
                *self.0.preview.borrow_mut() = Some(view::Preview { pages, paper, printing: self.0.deps.printer.available(), generation });
            }
            Effect::CheckUpdates => {
                let now = (self.0.deps.now_ms)();
                // The newer of the record on disk (another window may have asked meanwhile) and this
                // window's own (the file may not be writable).
                let record = crate::update::newest(store.load_update(), self.0.state.borrow().update.clone());
                let every = self.0.state.borrow().settings.update_every.ms();
                if !crate::update::due(&record, every, now) || self.0.checking.replace(true) {
                    return;
                }
                let (http, origin, post) = (self.0.deps.http.clone(), self.0.deps.share_origin.clone(), self.poster());
                self.0.deps.background.spawn(Box::new(move || {
                    let latest = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::update::fetch(&*http, &origin))).ok().flatten();
                    post(Event::UpdateChecked(crate::update::answered(&record, latest, now)));
                }));
            }
            Effect::SaveUpdate(record) => store.save_update(&record),
            Effect::OpenUpdatePage => {
                let page = self.0.state.borrow().update_note().map(|l| if self.lang() == Locale::It { l.page_it.clone() } else { l.page_en.clone() });
                if let Some(page) = page {
                    (self.0.deps.open_url)(&page);
                }
            }
        }
    }

    fn lang(&self) -> Locale {
        self.0.state.borrow().lang(self.0.deps.system_lang)
    }

    /// Runs `f` with this check's report and its receipt (language and time zone as on screen).
    fn with_receipt<T>(&self, f: impl FnOnce(&Report, &fettle_receipt::ReceiptModel) -> T) -> Option<T> {
        let report = self.0.state.borrow().current.as_ref()?.report.clone();
        let at = fettle_verdicts::fmt::parse_rfc3339(&report.collected_at)?;
        let (_, model) = fettle_receipt::receipt_for(&report, self.lang(), fettle_receipt::Zone::Offset((self.0.deps.zone)(at))).ok()?;
        Some(f(&report, &model))
    }

    /// Queues an event for the running pump (effects can't dispatch re-entrantly).
    fn post_now(&self, event: Event) {
        self.0.queue.lock().expect("event queue").push_back(event);
    }

    /// The window's callbacks become events.
    fn wire(&self) {
        let w = &self.0.window;
        let weak = Rc::downgrade(&self.0);
        let on = move |event: Event| {
            if let Some(inner) = weak.upgrade() {
                Fettle(inner).dispatch(event);
            }
        };
        let send = |event: Event| {
            let on = on.clone();
            move || on(event.clone())
        };
        w.on_check(send(Event::Start));
        w.on_save_asked(send(Event::SaveAsked));
        w.on_copy_asked(send(Event::CopyAsked));
        w.on_print_asked(send(Event::PrintAsked));
        w.on_print_confirmed(send(Event::PrintConfirmed));
        w.on_open_settings(send(Event::Open(Panel::Settings)));
        w.on_open_share(send(Event::Open(Panel::Share)));
        w.on_open_links(send(Event::Open(Panel::Links)));
        w.on_copy_link(send(Event::CopyLink));
        w.on_copy_delete_link(send(Event::CopyDeleteLink));
        w.on_run_deep(send(Event::DeepAsked));
        w.on_open_link(send(Event::OpenLink));
        w.on_open_update(send(Event::OpenUpdate));
        let o = on.clone();
        w.on_create_link(move |expiry, locked, password| {
            let expires = match expiry.as_str() {
                "7d" => crate::share::Expiry::Week,
                "1y" => crate::share::Expiry::Year,
                _ => crate::share::Expiry::Month,
            };
            // Locked means a password, even an empty one: it's refused as too short, never sent open.
            let password = locked.then(|| crate::state::Secret(password.to_string()));
            o(Event::CreateLink { expires, password })
        });
        let o = on.clone();
        w.on_delete_link(move |id| o(Event::DeleteLink(id.to_string())));
        w.on_check_links(send(Event::CheckLinks));
        w.on_back(send(Event::Back));
        let o = on.clone();
        w.on_theme_chosen(move |v| {
            o(Event::SetTheme(match v.as_str() {
                "light" => ThemeChoice::Light,
                "dark" => ThemeChoice::Dark,
                "oled" => ThemeChoice::Oled,
                _ => ThemeChoice::Auto,
            }))
        });
        let o = on.clone();
        w.on_lang_chosen(move |v| o(Event::SetLang(if v == "auto" { None } else { Some(Locale::from_tag(&v)) })));
        let o = on.clone();
        w.on_remember_toggled(move |v| o(Event::SetRemember(v)));
        let o = on.clone();
        w.on_private_toggled(move |v| o(Event::SetShowPrivate(v)));
        let o = on.clone();
        w.on_updates_toggled(move |v| o(Event::SetUpdates(v)));
        let o = on.clone();
        w.on_update_every_chosen(move |v| {
            o(Event::SetUpdateEvery(match v.as_str() {
                "hourly" => crate::state::UpdateEvery::Hourly,
                "weekly" => crate::state::UpdateEvery::Weekly,
                _ => crate::state::UpdateEvery::Daily,
            }))
        });
        #[cfg(windows)]
        {
            let weak = w.as_weak();
            w.on_theme_changed(move |dark, oled| {
                if let Some(w) = weak.upgrade() {
                    crate::win::tint_title_bar(w.window(), dark, oled);
                }
            });
        }
    }
}
