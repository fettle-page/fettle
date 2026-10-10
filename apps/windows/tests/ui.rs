//! The app's window, driven like a screen reader would: elements found by their accessible labels.
//! Each test thread gets its own Slint testing backend (no real window, mock time). Background
//! work (the scan) waits in a queue until the test runs it, so every in-between state can be seen.

use fettle_facts::Report;
use fettle_verdicts::Locale;
use fettle_windows::app::{Background, Clipboard, Dialogs, Job, Printer, Scanner};
use fettle_windows::print::layout::PageSize;
use fettle_windows::share::{Http, Response};
use fettle_windows::state::{PrivateCard, PrivateKind};
use fettle_windows::store::{FileStore, Store};
use fettle_windows::{Deps, Fettle};
use i_slint_backend_testing::ElementHandle;
use slint::ComponentHandle;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

thread_local! {
    static READY: Cell<bool> = const { Cell::new(false) };
}

/// Jobs wait here until the test runs them.
#[derive(Default)]
struct Manual {
    jobs: RefCell<Vec<Job>>,
}

impl Background for Manual {
    fn spawn(&self, job: Job) {
        self.jobs.borrow_mut().push(job);
    }
}

impl Manual {
    fn run_all(&self, app: &Fettle) {
        let jobs: Vec<Job> = self.jobs.borrow_mut().drain(..).collect();
        for job in jobs {
            job();
        }
        app.pump();
        // Let the receipt's feed and print-in animations finish (mock time).
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_secs(2));
    }
}

/// Reports every source, then the given report.
struct FakeScanner(Report);

impl Scanner for FakeScanner {
    fn scan(&self, on_source: &dyn Fn(&str)) -> Report {
        for (_, sources) in fettle_windows::sections::SECTION_SOURCES {
            for s in *sources {
                on_source(s);
            }
        }
        self.0.clone()
    }
}

fn dump_report() -> Report {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/fettle-receipt/tests/parity/cases");
    let file = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).find(|p| p.file_name().unwrap().to_string_lossy().starts_with("dump-dev-workstation-user")).unwrap();
    let case: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
    serde_json::from_value(case["report"].clone()).unwrap()
}

/// A fresh scratch folder under target/ (never Windows' Temp); each run clears the last run's.
fn temp_dir() -> PathBuf {
    static ROOT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    static N: AtomicUsize = AtomicUsize::new(0);
    let root = ROOT.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fettle-ui");
        let _ = std::fs::remove_dir_all(&root);
        root
    });
    root.join(N.fetch_add(1, Ordering::Relaxed).to_string())
}

struct Harness {
    app: Fettle,
    background: Rc<Manual>,
}

fn harness_in(lang: Locale, dir: PathBuf) -> Harness {
    harness_with(lang, dir, |_| {})
}

/// A harness whose dependencies the test can still change (fake dialogs, clipboard…).
fn harness_with(lang: Locale, dir: PathBuf, change: impl FnOnce(&mut Deps)) -> Harness {
    if !READY.get() {
        i_slint_backend_testing::init_no_event_loop();
        READY.set(true);
    }
    let background = Rc::new(Manual::default());
    let deps = Deps {
        system_lang: lang,
        scanner: Arc::new(FakeScanner(dump_report())),
        store: Rc::new(FileStore::new(dir)),
        background: background.clone(),
        wake: Arc::new(|| {}),
        zone: Arc::new(|_| 120),
        now_ms: Arc::new(|| fettle_verdicts::fmt::parse_rfc3339("2026-10-09T12:00:00Z").unwrap()),
        private: Arc::new(|| PrivateCard { pc_name: Some("DESKTOP-FETTLE".into()), items: vec![(PrivateKind::Board, "SN-BOARD-123".into())] }),
        dialogs: Rc::new(FakeDialogs::default()),
        clipboard: Rc::new(FakeClipboard::default()),
        printer: Rc::new(FakePrinter::default()),
        http: Arc::new(FakeServer::default()),
        share_origin: "https://fettle.page".into(),
        open_url: Rc::new(|_| true),
        relaunch: Arc::new(|_| Ok(())),
        system_dark: Rc::new(|| None),
        // A4, whatever this PC's region says (a US machine measures in inches and would get Letter).
        paper: Rc::new(|| PageSize::A4),
    };
    let mut deps = deps;
    change(&mut deps);
    let app = Fettle::new(deps).expect("the window builds");
    // The window's opening size (the testing backend otherwise lays out at the minimum).
    app.window().window().set_size(slint::LogicalSize::new(1240.0, 720.0));
    Harness { app, background }
}

fn harness(lang: Locale) -> Harness {
    harness_in(lang, temp_dir())
}

fn has(app: &Fettle, label: &str) -> bool {
    ElementHandle::find_by_accessible_label(app.window(), label).next().is_some()
}

fn press(app: &Fettle, label: &str) {
    let button = ElementHandle::find_by_accessible_label(app.window(), label).next().unwrap_or_else(|| panic!("no element labelled {label:?}"));
    button.invoke_accessible_default_action();
}

#[test]
fn first_screen_offers_the_check_in_english() {
    let h = harness(Locale::En);
    assert!(has(&h.app, "Check this PC"));
    assert!(has(&h.app, "A blank receipt: what Fettle will check"));
}

#[test]
fn first_screen_offers_the_check_in_italian() {
    let h = harness(Locale::It);
    assert!(has(&h.app, "Controlla questo PC"));
    assert!(!has(&h.app, "Check this PC"));
}

#[test]
fn checking_this_pc_shows_progress_then_the_receipt() {
    let h = harness(Locale::En);
    press(&h.app, "Check this PC");
    // The scan waits in the queue: the checking list is up, nothing ticked yet.
    assert!(has(&h.app, "0 of 7 sections done"));
    assert!(!has(&h.app, "Check this PC"));
    h.background.run_all(&h.app);
    assert!(has(&h.app, "In fine fettle."), "the receipt's summary");
    assert!(has(&h.app, "Nothing to do right now."), "the answer in the panel");
    assert!(has(&h.app, "Share a link"));
}

#[test]
fn the_last_check_comes_back_faded_at_the_next_launch() {
    let dir = temp_dir();
    let first = harness_in(Locale::En, dir.clone());
    press(&first.app, "Check this PC");
    first.background.run_all(&first.app);
    assert!(FileStore::new(dir.clone()).load_last().is_some(), "remembered on this PC");
    drop(first);
    let again = harness_in(Locale::En, dir);
    assert!(has(&again.app, "Last time: in fine fettle."));
    assert!(has(&again.app, "Check again"));
    // Its label sits on the mat beside the paper, never over the receipt's lines.
    let label = ElementHandle::find_by_accessible_label(again.app.window(), "Last check · 3 days ago").next().expect("the label on the mat");
    let mat_w = 1240.0 - 373.0;
    let paper_right = (mat_w - 372.0) / 2.0 + 372.0;
    assert!(label.absolute_position().x >= paper_right, "{:?}", label.absolute_position());
    assert!(label.absolute_position().x + label.size().width <= mat_w, "{:?} {:?}", label.absolute_position(), label.size());
}


#[test]
fn the_label_stays_on_the_mat_while_the_receipt_scrolls() {
    let dir = temp_dir();
    let first = harness_in(Locale::En, dir.clone());
    press(&first.app, "Check this PC");
    first.background.run_all(&first.app);
    drop(first);
    let again = harness_in(Locale::En, dir);
    let label = || ElementHandle::find_by_accessible_label(again.app.window(), "Last check · 3 days ago").next().expect("the label on the mat");
    let before = label().absolute_position();
    // Below the printer (its top at 12 px, 24 px tall), never under it.
    assert!(before.y >= 12.0 + 24.0, "{before:?}");
    again.app.window().set_mat_scroll(-300.0);
    again.app.pump();
    assert_eq!(label().absolute_position(), before, "it stays where it was put");
}

#[test]
fn the_label_fits_beside_the_receipt_in_the_smallest_window() {
    let dir = temp_dir();
    let first = harness_in(Locale::It, dir.clone());
    press(&first.app, "Controlla questo PC");
    first.background.run_all(&first.app);
    drop(first);
    let again = harness_in(Locale::It, dir);
    again.app.window().window().set_size(slint::LogicalSize::new(960.0, 600.0));
    again.app.pump();
    let label = ElementHandle::find_by_accessible_label(again.app.window(), "Ultimo controllo · 3 giorni fa").next().expect("the label on the mat");
    let mat_w = 960.0 - 373.0;
    let paper_right = (mat_w - 372.0) / 2.0 + 372.0;
    let (at, size) = (label.absolute_position(), label.size());
    assert!(at.x >= paper_right && at.x + size.width <= mat_w, "{at:?} {size:?}");
    // Wrapped, its lines still fit inside it.
    assert!(size.height >= 40.0, "{size:?}");
}

fn checked(app: &Fettle, label: &str) -> Option<bool> {
    ElementHandle::find_by_accessible_label(app.window(), label).next().and_then(|e| e.accessible_checked())
}

#[test]
fn choosing_italian_relabels_the_app() {
    let h = harness(Locale::En);
    press(&h.app, "Settings");
    press(&h.app, "Italiano");
    assert!(has(&h.app, "Impostazioni"));
    press(&h.app, "Indietro");
    assert!(has(&h.app, "Controlla questo PC"));
}

#[test]
fn choosing_a_theme_changes_the_colours() {
    let h = harness(Locale::En);
    press(&h.app, "Settings");
    press(&h.app, "Dark");
    assert!(h.app.window().global::<fettle_windows::Theme>().get_dark());
    press(&h.app, "Light");
    assert!(!h.app.window().global::<fettle_windows::Theme>().get_dark());
    assert_eq!(checked(&h.app, "Light"), Some(true));
}

#[test]
fn turning_remember_off_deletes_the_stored_check() {
    let dir = temp_dir();
    let h = harness_in(Locale::En, dir.clone());
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    assert!(FileStore::new(dir.clone()).load_last().is_some());
    press(&h.app, "Settings");
    assert_eq!(checked(&h.app, "Remember the last check on this PC"), Some(true));
    press(&h.app, "Remember the last check on this PC");
    assert_eq!(checked(&h.app, "Remember the last check on this PC"), Some(false));
    assert!(!dir.join("last-report.json").exists());
}

#[test]
fn the_privacy_switch_shows_the_details_only_while_on() {
    let h = harness(Locale::En);
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    assert!(!h.app.window().get_private_shown());
    press(&h.app, "Settings");
    press(&h.app, "Show this PC's name and serial numbers on this screen");
    h.background.run_all(&h.app);
    press(&h.app, "Back");
    // The card sits below the fold of the side panel's scroll area, where the testing API (unlike
    // a screen reader) doesn't look; its rows are read from the window instead.
    assert!(h.app.window().get_private_shown());
    assert_eq!(private_rows(&h.app), vec![("PC name".to_string(), "DESKTOP-FETTLE".to_string()), ("Motherboard serial".into(), "SN-BOARD-123".into())]);
    press(&h.app, "Settings");
    press(&h.app, "Show this PC's name and serial numbers on this screen");
    press(&h.app, "Back");
    assert!(!h.app.window().get_private_shown());
    assert!(private_rows(&h.app).is_empty());
}

#[test]
fn details_that_cant_be_read_still_show_the_card() {
    let h = harness_with(Locale::En, temp_dir(), |deps| deps.private = Arc::new(|| panic!("WMI broke")));
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    press(&h.app, "Settings");
    press(&h.app, "Show this PC's name and serial numbers on this screen");
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| h.background.run_all(&h.app)));
    h.app.pump();
    press(&h.app, "Back");
    // The card says Windows reports nothing, rather than never appearing.
    assert!(h.app.window().get_private_shown());
    assert!(private_rows(&h.app).is_empty());
}

fn private_rows(app: &Fettle) -> Vec<(String, String)> {
    use slint::Model;
    app.window().get_private_rows().iter().map(|r| (r.label.to_string(), r.value.to_string())).collect()
}

#[test]
fn settings_credit_slint_with_its_own_badge() {
    let h = harness(Locale::En);
    // Tall enough that the About part is in view (the testing API only finds what's on screen).
    h.app.window().window().set_size(slint::LogicalSize::new(1240.0, 2000.0));
    press(&h.app, "Settings");
    assert!(has(&h.app, "#MadeWithSlint"), "Slint's AboutSlint badge");
    assert!(has(&h.app, "Fonts: Bricolage Grotesque and IBM Plex Mono (OFL)"));
}

#[test]
fn settings_survive_a_restart() {
    let dir = temp_dir();
    let first = harness_in(Locale::En, dir.clone());
    press(&first.app, "Settings");
    press(&first.app, "Italiano");
    press(&first.app, "OLED");
    drop(first);
    let again = harness_in(Locale::En, dir);
    assert!(has(&again.app, "Controlla questo PC"));
    assert!(again.app.window().global::<fettle_windows::Theme>().get_oled());
}

#[derive(Default)]
struct FakeDialogs {
    answer: RefCell<Option<PathBuf>>,
    asked: RefCell<Vec<String>>,
}

impl Dialogs for FakeDialogs {
    fn save(&self, _owner: &slint::Window, suggested: &str, _filters: &[(String, Vec<String>)]) -> Option<PathBuf> {
        self.asked.borrow_mut().push(suggested.to_string());
        self.answer.borrow().clone()
    }
}

#[derive(Default)]
struct FakeClipboard(RefCell<Option<String>>);

impl Clipboard for FakeClipboard {
    fn set_text(&self, text: &str) -> bool {
        *self.0.borrow_mut() = Some(text.to_string());
        true
    }
}

struct Exports {
    h: Harness,
    dialogs: Rc<FakeDialogs>,
    clipboard: Rc<FakeClipboard>,
    dir: PathBuf,
}

/// A checked PC with fake dialogs and clipboard.
fn exports() -> Exports {
    let dir = temp_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let dialogs = Rc::new(FakeDialogs::default());
    let clipboard = Rc::new(FakeClipboard::default());
    let (d, c) = (dialogs.clone(), clipboard.clone());
    let h = harness_with(Locale::En, dir.join("store"), move |deps| {
        deps.dialogs = d;
        deps.clipboard = c;
    });
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    Exports { h, dialogs, clipboard, dir }
}

fn dock_status(app: &Fettle) -> String {
    app.window().get_dock_status().to_string()
}

#[test]
fn save_writes_the_receipt_as_a_pdf() {
    let e = exports();
    let path = e.dir.join("receipt.pdf");
    *e.dialogs.answer.borrow_mut() = Some(path.clone());
    press(&e.h.app, "Save");
    assert!(e.dialogs.asked.borrow()[0].starts_with("fettle-2026-10-0"), "{:?}", e.dialogs.asked.borrow());
    assert!(std::fs::read(&path).unwrap().starts_with(b"%PDF"));
    assert_eq!(dock_status(&e.h.app), "Saved.");
}

#[test]
fn save_writes_the_report_as_json_when_json_is_chosen() {
    let e = exports();
    let path = e.dir.join("report.json");
    *e.dialogs.answer.borrow_mut() = Some(path.clone());
    press(&e.h.app, "Save");
    let saved: Report = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(saved.engine_version.as_deref(), Some(fettle_verdicts::ENGINE_VERSION));
    assert_eq!(saved.facts, dump_report().facts, "the same redacted report");
}

#[test]
fn a_cancelled_save_says_nothing() {
    let e = exports();
    press(&e.h.app, "Save");
    assert_eq!(dock_status(&e.h.app), "");
}

#[test]
fn a_failed_save_says_so() {
    let e = exports();
    *e.dialogs.answer.borrow_mut() = Some(e.dir.join("no-such-folder").join("x.pdf"));
    press(&e.h.app, "Save");
    assert_eq!(dock_status(&e.h.app), "Something went wrong. Try again.");
}

#[test]
fn copy_puts_the_markdown_on_the_clipboard() {
    let e = exports();
    press(&e.h.app, "Copy");
    let text = e.clipboard.0.borrow().clone().expect("copied");
    assert!(text.starts_with("**Fettle** · "), "{text}");
    assert!(text.contains("**In fine fettle.**"));
    assert_eq!(dock_status(&e.h.app), "Copied. Paste it where you're asking for help.");
}

#[derive(Default)]
struct FakePrinter(RefCell<Vec<String>>);

impl Printer for FakePrinter {
    fn print(&self, _owner: &slint::Window, model: &fettle_receipt::ReceiptModel) -> Result<bool, String> {
        self.0.borrow_mut().push(model.summary.clone());
        Ok(true)
    }
}

/// A PC where Windows can't print (the Print Spooler is off, or no printer is installed).
struct NoPrinting;

impl Printer for NoPrinting {
    fn available(&self) -> bool {
        false
    }
    fn print(&self, _owner: &slint::Window, _model: &fettle_receipt::ReceiptModel) -> Result<bool, String> {
        panic!("never asked to print");
    }
}

fn no_printing() -> Exports {
    let dir = temp_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let dialogs = Rc::new(FakeDialogs::default());
    let d = dialogs.clone();
    let h = harness_with(Locale::En, dir.join("store"), move |deps| {
        deps.dialogs = d;
        deps.printer = Rc::new(NoPrinting);
    });
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    Exports { h, dialogs, clipboard: Rc::new(FakeClipboard::default()), dir }
}

#[test]
fn when_windows_cant_print_print_saves_a_pdf_instead() {
    let e = no_printing();
    let path = e.dir.join("receipt.pdf");
    *e.dialogs.answer.borrow_mut() = Some(path.clone());
    press(&e.h.app, "Print");
    assert!(has(&e.h.app, "Windows can't print on this PC right now, so this saves a PDF instead."));
    press(&e.h.app, "Save as PDF");
    assert_eq!(e.dialogs.asked.borrow().len(), 1, "Save opened");
    assert!(std::fs::read(&path).unwrap().starts_with(b"%PDF"));
    assert_eq!(dock_status(&e.h.app), "Windows can't print on this PC right now, so Fettle saved the receipt as a PDF instead.");
}

#[test]
fn when_windows_cant_print_and_save_is_cancelled_it_still_says_why() {
    let e = no_printing();
    press(&e.h.app, "Print");
    press(&e.h.app, "Save as PDF");
    assert_eq!(dock_status(&e.h.app), "Windows can't print on this PC right now. Save makes the same receipt as a PDF.");
}

#[test]
fn print_sends_the_receipt_to_the_printer() {
    let printer = Rc::new(FakePrinter::default());
    let p = printer.clone();
    let h = harness_with(Locale::It, temp_dir(), move |deps| deps.printer = p);
    press(&h.app, "Controlla questo PC");
    h.background.run_all(&h.app);
    press(&h.app, "Stampa");
    assert!(printer.0.borrow().is_empty(), "the preview comes first");
    assert!(has(&h.app, "Anteprima di stampa"));
    press(&h.app, "Stampa…");
    assert_eq!(*printer.0.borrow(), vec!["In gran forma.".to_string()], "the receipt, in the app's language");
}

fn preview_pages(app: &Fettle) -> Vec<slint::Image> {
    use slint::Model;
    app.window().get_preview_pages().iter().collect()
}

#[test]
fn the_preview_uses_the_paper_windows_is_set_up_for() {
    // Letter is shorter than A4, so the same receipt takes a page more.
    let h = harness_with(Locale::En, temp_dir(), |deps| deps.paper = Rc::new(|| PageSize::Letter));
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    press(&h.app, "Print");
    assert_eq!(preview_pages(&h.app).len(), 3);
    let label = h.app.window().get_preview_label().to_string();
    assert!(label.starts_with("3 pages · ") && label.contains("Letter"), "{label}");
}

#[test]
fn the_preview_shows_the_pages_as_they_will_print() {
    let h = harness(Locale::En);
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    press(&h.app, "Print");
    let pages = preview_pages(&h.app);
    assert_eq!(pages.len(), 2, "the dump's receipt is two pages");
    let size = pages[0].size();
    // Portrait paper (A4 or Letter), drawn with ink on it.
    assert!(size.width > 300 && size.height as f32 / size.width as f32 > 1.25, "{size:?}");
    let label = h.app.window().get_preview_label().to_string();
    assert!(label.starts_with("2 pages · "), "{label}");
    key(&h.app, slint::platform::Key::Escape);
    assert!(has(&h.app, "Share a link"), "Esc goes back to the receipt");
    assert!(preview_pages(&h.app).is_empty(), "the pages aren't kept");
}

/// A share server that answers like the Worker and keeps what it was sent.
#[derive(Default)]
struct FakeServer(std::sync::Mutex<Vec<(String, String, Option<serde_json::Value>)>>);

impl Http for FakeServer {
    fn send(&self, method: &str, url: &str, _headers: &[(&str, String)], body: Option<Vec<u8>>) -> Result<Response, ()> {
        // The update check's own file isn't part of sharing: not logged, not found.
        if url.ends_with("/app/latest.json") {
            return Ok(Response { status: 404, headers: Default::default(), body: vec![] });
        }
        self.0.lock().unwrap().push((method.into(), url.into(), body.map(|b| serde_json::from_slice(&b).unwrap())));
        let created = serde_json::json!({ "id": "m0123456789abcdefg", "url": "https://fettle.page/r/m0123456789abcdefg", "deleteUrl": "https://fettle.page/r/m0123456789abcdefg/delete#tok", "deleteToken": "tok", "expiresAt": "2026-11-08T12:00:00.000Z", "locked": false });
        Ok(match method {
            "POST" => Response { status: 201, headers: Default::default(), body: created.to_string().into_bytes() },
            _ => Response { status: 204, headers: Default::default(), body: vec![] },
        })
    }
}

fn share_harness() -> (Harness, Arc<FakeServer>) {
    let server = Arc::new(FakeServer::default());
    let s = server.clone();
    let h = harness_with(Locale::En, temp_dir(), move |deps| deps.http = s);
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    (h, server)
}

fn value_of(app: &Fettle, label: &str) -> String {
    ElementHandle::find_by_accessible_label(app.window(), label).next().and_then(|e| e.accessible_value()).map(|v| v.to_string()).unwrap_or_default()
}

#[test]
fn sharing_makes_a_link_and_remembers_it() {
    let (h, server) = share_harness();
    press(&h.app, "Share a link");
    assert!(has(&h.app, "Share this receipt"));
    press(&h.app, "Create the link");
    h.background.run_all(&h.app);
    assert!(has(&h.app, "Your link is ready."));
    assert_eq!(value_of(&h.app, "Link"), "https://fettle.page/r/m0123456789abcdefg");
    let sent = server.0.lock().unwrap()[0].2.clone().unwrap();
    assert_eq!(sent["expires"], "30d", "a month by default");
    assert_eq!(sent["report"]["engine_version"], fettle_verdicts::ENGINE_VERSION);
    assert!(sent.get("password").is_none());
    press(&h.app, "Back");
    assert!(has(&h.app, "Shared links · 1"));
}

#[test]
fn a_locked_link_sends_the_password_once_and_shows_it_nowhere() {
    let (h, server) = share_harness();
    press(&h.app, "Share a link");
    press(&h.app, "Lock it with a password");
    let field = ElementHandle::find_by_accessible_label(h.app.window(), "Password (at least 8 characters)").next().expect("the password field");
    field.set_accessible_value("hunter22");
    press(&h.app, "Create the link");
    h.background.run_all(&h.app);
    let sent = server.0.lock().unwrap()[0].2.clone().unwrap();
    assert_eq!(sent["password"], "hunter22");
    assert!(!h.app.window().get_share_json().contains("hunter22"), "not in the JSON shown");
}

#[test]
fn a_too_short_password_is_refused_before_sending() {
    let (h, server) = share_harness();
    press(&h.app, "Share a link");
    press(&h.app, "Lock it with a password");
    ElementHandle::find_by_accessible_label(h.app.window(), "Password (at least 8 characters)").next().unwrap().set_accessible_value("short");
    press(&h.app, "Create the link");
    h.background.run_all(&h.app);
    assert!(has(&h.app, "The password needs at least 8 characters."));
    assert!(server.0.lock().unwrap().is_empty());
}

#[test]
fn a_shared_link_can_be_deleted_from_this_pc() {
    let (h, server) = share_harness();
    press(&h.app, "Share a link");
    press(&h.app, "Create the link");
    h.background.run_all(&h.app);
    press(&h.app, "Back");
    press(&h.app, "Shared links · 1");
    assert!(has(&h.app, "My shared links"));
    press(&h.app, "Delete");
    h.background.run_all(&h.app);
    assert!(has(&h.app, "Deleted. The link no longer works."));
    assert!(has(&h.app, "No shared links on this PC."));
    let (method, url, _) = server.0.lock().unwrap()[1].clone();
    assert_eq!((method.as_str(), url.as_str()), ("DELETE", "https://fettle.page/api/reports/m0123456789abcdefg"));
}

/// A transport that blows up: the panel must still come back with a message.
struct PanickingServer;

impl Http for PanickingServer {
    fn send(&self, _: &str, _: &str, _: &[(&str, String)], _: Option<Vec<u8>>) -> Result<Response, ()> {
        panic!("transport exploded")
    }
}

#[test]
fn a_crashing_upload_still_ends_with_a_message() {
    let h = harness_with(Locale::En, temp_dir(), |deps| deps.http = Arc::new(PanickingServer));
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    press(&h.app, "Share a link");
    press(&h.app, "Create the link");
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| h.background.run_all(&h.app)));
    h.app.pump();
    assert!(has(&h.app, "The link couldn't be created. Try again."));
    assert!(has(&h.app, "Create the link"), "and it can be tried again");
}

#[test]
fn a_declined_deep_check_keeps_the_quick_receipt_and_says_so() {
    let asked = Arc::new(AtomicUsize::new(0));
    let a = asked.clone();
    let h = harness_with(Locale::En, temp_dir(), move |deps| {
        deps.relaunch = Arc::new(move |_owner| {
            a.fetch_add(1, Ordering::Relaxed);
            Err(fettle_windows::state::DeepError::Cancelled)
        })
    });
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    press(&h.app, "Run a Deep check");
    // Windows' prompt waits off the window's thread; meanwhile the button can't ask twice.
    assert_eq!(asked.load(Ordering::Relaxed), 0);
    let button = ElementHandle::find_by_accessible_label(h.app.window(), "Run a Deep check").next().unwrap();
    assert_eq!(button.accessible_enabled(), Some(false));
    h.background.run_all(&h.app);
    assert_eq!(asked.load(Ordering::Relaxed), 1);
    assert_eq!(button.accessible_enabled(), Some(true));
    assert!(has(&h.app, "The Deep check was cancelled. This is still the quick check."));
    assert!(has(&h.app, "In fine fettle."), "the quick receipt stays");
}

/// Works the first time, then breaks.
struct OnceScanner(Report, AtomicUsize);

impl Scanner for OnceScanner {
    fn scan(&self, _on_source: &dyn Fn(&str)) -> Report {
        if self.1.fetch_add(1, Ordering::Relaxed) > 0 {
            panic!("the scan broke");
        }
        self.0.clone()
    }
}

#[test]
fn a_failed_re_check_keeps_the_receipt_and_says_so_above_it() {
    let h = harness_with(Locale::En, temp_dir(), |deps| deps.scanner = Arc::new(OnceScanner(dump_report(), AtomicUsize::new(0))));
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    press(&h.app, "Check again");
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| h.background.run_all(&h.app)));
    h.app.pump();
    assert!(has(&h.app, "The check couldn't finish. Try again."));
    assert!(has(&h.app, "In fine fettle."), "the receipt stays");
    assert!(has(&h.app, "Share a link"), "and can still be shared");
}

// Keys, dispatched to the app's own (test) window: nothing reaches Windows or other apps.
fn key(app: &Fettle, k: impl Into<slint::SharedString> + Clone) {
    use slint::platform::WindowEvent;
    let w = app.window().window();
    w.dispatch_event(WindowEvent::KeyPressed { text: k.clone().into() });
    w.dispatch_event(WindowEvent::KeyReleased { text: k.into() });
}

/// A mouse click in the middle of an element, dispatched to the app's own (test) window.
fn click(app: &Fettle, e: &ElementHandle) {
    use slint::platform::{PointerEventButton, WindowEvent};
    let (pos, size) = (e.absolute_position(), e.size());
    let position = slint::LogicalPosition::new(pos.x + size.width / 2.0, pos.y + size.height / 2.0);
    let w = app.window().window();
    w.dispatch_event(WindowEvent::PointerPressed { position, button: PointerEventButton::Left });
    w.dispatch_event(WindowEvent::PointerReleased { position, button: PointerEventButton::Left });
}

fn ctrl(app: &Fettle, letter: &str) {
    use slint::platform::{Key, WindowEvent};
    let w = app.window().window();
    w.dispatch_event(WindowEvent::KeyPressed { text: Key::Control.into() });
    key(app, letter.to_string());
    w.dispatch_event(WindowEvent::KeyReleased { text: Key::Control.into() });
}

#[test]
fn enter_starts_the_check() {
    let h = harness(Locale::En);
    key(&h.app, slint::platform::Key::Return);
    assert!(has(&h.app, "0 of 7 sections done"));
}

#[test]
fn shortcuts_work_after_a_check() {
    let printer = Rc::new(FakePrinter::default());
    let dialogs = Rc::new(FakeDialogs::default());
    let (p, d) = (printer.clone(), dialogs.clone());
    let h = harness_with(Locale::En, temp_dir(), move |deps| {
        deps.printer = p;
        deps.dialogs = d;
    });
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    ctrl(&h.app, "p");
    assert!(has(&h.app, "Print preview"), "Ctrl+P opens the preview");
    ctrl(&h.app, "p");
    assert_eq!(printer.0.borrow().len(), 1, "Ctrl+P again prints");
    ctrl(&h.app, "s");
    assert_eq!(dialogs.asked.borrow().len(), 1, "Ctrl+S saves");
    ctrl(&h.app, "r");
    assert!(has(&h.app, "0 of 7 sections done"), "Ctrl+R checks again");
}

#[test]
fn print_and_save_shortcuts_wait_while_a_view_is_open() {
    let printer = Rc::new(FakePrinter::default());
    let dialogs = Rc::new(FakeDialogs::default());
    let (p, d) = (printer.clone(), dialogs.clone());
    let h = harness_with(Locale::En, temp_dir(), move |deps| {
        deps.printer = p;
        deps.dialogs = d;
    });
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    for view in ["Share a link", "Settings"] {
        press(&h.app, view);
        ctrl(&h.app, "p");
        ctrl(&h.app, "s");
        press(&h.app, "Back");
    }
    assert_eq!(printer.0.borrow().len(), 0, "no printing from a view");
    assert_eq!(dialogs.asked.borrow().len(), 0, "no saving from a view");
}

#[test]
fn escape_leaves_a_view_but_not_while_typing() {
    let h = harness(Locale::En);
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    press(&h.app, "Settings");
    key(&h.app, slint::platform::Key::Escape);
    assert!(!has(&h.app, "Back"), "Esc closed Settings");
    press(&h.app, "Share a link");
    press(&h.app, "Lock it with a password");
    let field = ElementHandle::find_by_accessible_label(h.app.window(), "Password (at least 8 characters)").next().unwrap();
    click(&h.app, &field);
    field.set_accessible_value("half typed");
    h.app.window().window().dispatch_event(slint::platform::WindowEvent::KeyPressed { text: slint::platform::Key::Escape.into() });
    assert!(has(&h.app, "Share this receipt"), "Esc in the field belongs to the field");
}

#[test]
fn every_control_has_a_name_for_screen_readers() {
    use i_slint_backend_testing::AccessibleRole as R;
    let h = harness(Locale::En);
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    for view in ["Settings", "Back", "Share a link"] {
        let controls = i_slint_backend_testing::ElementQuery::from_root(h.app.window())
            .match_descendants()
            .match_predicate(|e| matches!(e.accessible_role(), Some(R::Button | R::Switch | R::Checkbox | R::Tab | R::TextInput)))
            .find_all();
        assert!(!controls.is_empty());
        for c in controls {
            assert!(c.accessible_label().is_some_and(|l| !l.trim().is_empty()), "a {:?} without a name before {view}", c.accessible_role());
        }
        press(&h.app, view);
    }
}

fn selected_tab(app: &Fettle) -> String {
    use i_slint_backend_testing::AccessibleRole as R;
    i_slint_backend_testing::ElementQuery::from_root(app.window())
        .match_descendants()
        .match_predicate(|e| e.accessible_role() == Some(R::Tab) && e.accessible_checked() == Some(true))
        .find_first()
        .and_then(|e| e.accessible_label())
        .map(|l| l.to_string())
        .unwrap_or_default()
}

#[test]
fn arrow_keys_move_between_the_numbers_tabs() {
    let h = harness(Locale::En);
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    assert_eq!(selected_tab(&h.app), "Storage");
    let tab = ElementHandle::find_by_accessible_label(h.app.window(), "Storage").find(|e| e.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::Tab)).unwrap();
    click(&h.app, &tab);
    key(&h.app, slint::platform::Key::RightArrow);
    assert_eq!(selected_tab(&h.app), "Memory");
    key(&h.app, slint::platform::Key::LeftArrow);
    key(&h.app, slint::platform::Key::LeftArrow);
    assert_eq!(selected_tab(&h.app), "Graphics", "wraps around");
}

#[test]
fn tabbing_follows_the_reading_order_and_space_toggles_a_switch() {
    let h = harness(Locale::En);
    press(&h.app, "Settings");
    let remember = || ElementHandle::find_by_accessible_label(h.app.window(), "Remember the last check on this PC").next().unwrap().accessible_checked();
    assert_eq!(remember(), Some(true));
    // Opening Settings puts the keyboard on Back; then the four themes, the three languages, Remember.
    for _ in 0..8 {
        key(&h.app, slint::platform::Key::Tab);
    }
    key(&h.app, " ");
    assert_eq!(remember(), Some(false), "Remember was reached with Tab and toggled with Space");
}


// ---- Share links, folds, announcements and the keyboard ----

#[test]
fn an_empty_password_never_makes_an_open_link() {
    let (h, server) = share_harness();
    press(&h.app, "Share a link");
    press(&h.app, "Lock it with a password");
    press(&h.app, "Create the link");
    h.background.run_all(&h.app);
    assert!(has(&h.app, "The password needs at least 8 characters."));
    assert!(server.0.lock().unwrap().is_empty(), "nothing was sent");
}

#[test]
fn an_open_fold_stays_open_when_something_else_happens() {
    let h = harness(Locale::En);
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    let expanded = || ElementHandle::find_by_accessible_label(h.app.window(), "details: Storage").next().and_then(|e| e.accessible_expanded());
    press(&h.app, "details: Storage");
    assert_eq!(expanded(), Some(true));
    press(&h.app, "Copy");
    press(&h.app, "Settings");
    press(&h.app, "Back");
    assert_eq!(expanded(), Some(true), "the receipt wasn't rebuilt");
}

#[test]
fn status_messages_are_announced() {
    use i_slint_backend_testing::AccessibleLiveness as Live;
    let live = |app: &Fettle, label: &str| ElementHandle::find_by_accessible_label(app.window(), label).next().and_then(|e| e.accessible_live_region());
    let h = harness(Locale::En);
    press(&h.app, "Check this PC");
    assert_eq!(live(&h.app, "0 of 7 sections done"), Some(Live::Polite), "progress");
    h.background.run_all(&h.app);
    press(&h.app, "Copy");
    assert_eq!(live(&h.app, "Copied. Paste it where you're asking for help."), Some(Live::Polite), "the dock's message");
}

#[test]
fn the_json_fold_is_reachable_with_the_keyboard() {
    let (h, _) = share_harness();
    press(&h.app, "Share a link");
    let expanded = || ElementHandle::find_by_accessible_label(h.app.window(), "Advanced: show the JSON that will be sent").next().and_then(|e| e.accessible_expanded());
    assert_eq!(expanded(), Some(false));
    // Opening Share puts the keyboard on Back; then the three expiry choices, the lock switch, the fold.
    for _ in 0..5 {
        key(&h.app, slint::platform::Key::Tab);
    }
    key(&h.app, " ");
    assert_eq!(expanded(), Some(true));
}

/// The website's update file (and nothing else), counting who asks.
struct UpdateSite(&'static str, std::sync::Mutex<Vec<String>>);

impl Http for UpdateSite {
    fn send(&self, method: &str, url: &str, headers: &[(&str, String)], body: Option<Vec<u8>>) -> Result<Response, ()> {
        assert!(method == "GET" && headers.is_empty() && body.is_none(), "a plain GET");
        self.1.lock().unwrap().push(url.to_string());
        Ok(if url == "https://fettle.page/app/latest.json" { Response { status: 200, headers: Default::default(), body: self.0.as_bytes().to_vec() } } else { Response { status: 404, headers: Default::default(), body: vec![] } })
    }
}

const NEWER: &str = r#"{"version":"999.0.0","page":{"en":"https://fettle.page/download","it":"https://fettle.page/it/download"}}"#;

struct Updates {
    h: Harness,
    site: Arc<UpdateSite>,
    opened: Rc<RefCell<Vec<String>>>,
}

fn updates_in(lang: Locale, dir: PathBuf) -> Updates {
    let site = Arc::new(UpdateSite(NEWER, Default::default()));
    let opened = Rc::new(RefCell::new(Vec::new()));
    let (s, o) = (site.clone(), opened.clone());
    let h = harness_with(lang, dir, move |deps| {
        deps.http = s;
        deps.open_url = Rc::new(move |url| {
            o.borrow_mut().push(url.to_string());
            true
        });
    });
    Updates { h, site, opened }
}

#[test]
fn a_newer_version_shows_in_the_status_bar_with_a_way_to_get_it() {
    let u = updates_in(Locale::En, temp_dir());
    assert!(!has(&u.h.app, "Fettle 999.0.0 is out."), "not before the website answers");
    u.h.background.run_all(&u.h.app);
    assert_eq!(*u.site.1.lock().unwrap(), vec!["https://fettle.page/app/latest.json".to_string()]);
    assert!(has(&u.h.app, "Fettle 999.0.0 is out."));
    press(&u.h.app, "Download Fettle 999.0.0");
    assert_eq!(*u.opened.borrow(), vec!["https://fettle.page/download".to_string()]);
}

#[test]
fn in_italian_the_download_page_is_the_italian_one() {
    let u = updates_in(Locale::It, temp_dir());
    u.h.background.run_all(&u.h.app);
    assert!(has(&u.h.app, "È uscito Fettle 999.0.0."));
    press(&u.h.app, "Scarica Fettle 999.0.0");
    assert_eq!(*u.opened.borrow(), vec!["https://fettle.page/it/download".to_string()]);
}

#[test]
fn the_website_is_asked_at_most_once_a_day() {
    let now = fettle_verdicts::fmt::parse_rfc3339("2026-10-09T12:00:00Z").unwrap();
    let seeded = |hours_ago: i64| {
        let dir = temp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let record = serde_json::json!({ "checked_at": now - hours_ago * 3_600_000, "latest": { "version": "999.0.0", "page_en": "https://fettle.page/download", "page_it": "https://fettle.page/it/download" } });
        std::fs::write(dir.join("update.json"), record.to_string()).unwrap();
        dir
    };
    let recent = updates_in(Locale::En, seeded(1));
    recent.h.background.run_all(&recent.h.app);
    assert!(recent.site.1.lock().unwrap().is_empty(), "asked an hour ago: not again");
    assert!(has(&recent.h.app, "Fettle 999.0.0 is out."), "what it found then still shows");
    let stale = updates_in(Locale::En, seeded(25));
    stale.h.background.run_all(&stale.h.app);
    assert_eq!(stale.site.1.lock().unwrap().len(), 1, "a day has passed");
}

#[test]
fn with_updates_off_nothing_is_sent() {
    let dir = temp_dir();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("settings.json"), r#"{"updates":false}"#).unwrap();
    let u = updates_in(Locale::En, dir);
    u.h.background.run_all(&u.h.app);
    press(&u.h.app, "Check this PC");
    u.h.background.run_all(&u.h.app);
    assert!(u.site.1.lock().unwrap().is_empty());
    assert!(!has(&u.h.app, "Fettle 999.0.0 is out."));
}

fn seeded_update(hours_ago: i64, settings: &str) -> PathBuf {
    let now = fettle_verdicts::fmt::parse_rfc3339("2026-10-09T12:00:00Z").unwrap();
    let dir = temp_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let record = serde_json::json!({ "checked_at": now - hours_ago * 3_600_000, "latest": null });
    std::fs::write(dir.join("update.json"), record.to_string()).unwrap();
    std::fs::write(dir.join("settings.json"), settings).unwrap();
    dir
}

#[test]
fn checking_every_week_or_every_hour_is_respected() {
    let weekly = updates_in(Locale::En, seeded_update(48, r#"{"update_every":"weekly"}"#));
    weekly.h.background.run_all(&weekly.h.app);
    assert!(weekly.site.1.lock().unwrap().is_empty(), "two days into a week: not yet");
    let hourly = updates_in(Locale::En, seeded_update(2, r#"{"update_every":"hourly"}"#));
    hourly.h.background.run_all(&hourly.h.app);
    assert_eq!(hourly.site.1.lock().unwrap().len(), 1, "two hours into an hour: ask");
}

#[test]
fn settings_offer_how_often_to_check_while_the_check_is_on() {
    let dir = temp_dir();
    let h = harness_in(Locale::En, dir.clone());
    h.app.window().window().set_size(slint::LogicalSize::new(1240.0, 2000.0));
    press(&h.app, "Settings");
    assert_eq!(checked(&h.app, "Every day"), Some(true));
    press(&h.app, "Every week");
    assert_eq!(checked(&h.app, "Every week"), Some(true));
    assert_eq!(FileStore::new(dir).load_settings().update_every, fettle_windows::state::UpdateEvery::Weekly);
    press(&h.app, "Check for a new version");
    assert!(!has(&h.app, "Every week"), "no choice while the check is off");
}

#[test]
fn auto_follows_windows_light_or_dark_setting() {
    let h = harness(Locale::En);
    let theme = h.app.window().global::<fettle_windows::Theme>();
    theme.set_system_dark(true);
    assert!(theme.get_dark(), "Auto, Windows dark");
    theme.set_system_dark(false);
    assert!(!theme.get_dark(), "Auto, Windows light");
    press(&h.app, "Settings");
    press(&h.app, "Dark");
    assert!(theme.get_dark(), "Dark whatever Windows says");
}

#[test]
fn opening_the_json_fold_moves_nothing_above_it() {
    let (h, _) = share_harness();
    press(&h.app, "Share a link");
    let place = |label: &str| {
        let e = ElementHandle::find_by_accessible_label(h.app.window(), label).next().unwrap_or_else(|| panic!("no {label:?}"));
        (e.absolute_position().y, e.size().height)
    };
    let intro = "Anyone with the link can see the receipt on the left, exactly as it is. It never includes serial numbers, the PC's name or usernames.";
    let before = [place(intro), place("7 days"), place("Lock it with a password"), place("Advanced: show the JSON that will be sent")];
    press(&h.app, "Advanced: show the JSON that will be sent");
    h.app.pump();
    let after = [place(intro), place("7 days"), place("Lock it with a password"), place("Advanced: show the JSON that will be sent")];
    assert_eq!(before, after, "the view kept its spacing above the fold");
}

/// A tired laptop with three next steps (the website's sample report).
fn tired_report() -> Report {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../worker/test/fixtures/tired.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn every_next_step_sits_above_the_first_section() {
    let h = harness_with(Locale::En, temp_dir(), |deps| deps.scanner = Arc::new(FakeScanner(tired_report())));
    // Tall enough that the whole top of the receipt is on screen (the testing API only finds that).
    h.app.window().window().set_size(slint::LogicalSize::new(1240.0, 2400.0));
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    let at = |label: &str| {
        let e = ElementHandle::find_by_accessible_label(h.app.window(), label).next().unwrap_or_else(|| panic!("no {label:?}"));
        (e.absolute_position().y, e.absolute_position().y + e.size().height)
    };
    let step3 = at("NEXT STEP 3");
    let storage = at("STORAGE");
    assert!(step3.1 < storage.0, "step 3 ({step3:?}) runs into the Storage heading ({storage:?})");
}

#[test]
fn a_fresh_check_is_headed_this_check_not_last_check() {
    let h = harness(Locale::En);
    press(&h.app, "Check this PC");
    h.background.run_all(&h.app);
    assert!(has(&h.app, "THIS CHECK"));
    assert!(!has(&h.app, "LAST CHECK"), "that's the faded, remembered one");
    let it = harness(Locale::It);
    press(&it.app, "Controlla questo PC");
    it.background.run_all(&it.app);
    assert!(has(&it.app, "QUESTO CONTROLLO"));
}

#[test]
fn the_ready_link_comes_on_a_ticket_with_a_qr_code_and_both_links_copy() {
    let (h, _) = share_harness();
    press(&h.app, "Share a link");
    press(&h.app, "Create the link");
    h.background.run_all(&h.app);
    assert!(has(&h.app, "Your link is ready."));
    assert!(has(&h.app, "QR code of the link"), "a code to open it on a phone");
    assert_eq!(value_of(&h.app, "Link"), "https://fettle.page/r/m0123456789abcdefg", "the whole link, readable");
    press(&h.app, "Copy the delete link");
    assert!(has(&h.app, "Delete link copied."));
    press(&h.app, "Copy the link");
    assert!(has(&h.app, "Copied."));
    press(&h.app, "Done");
    assert!(has(&h.app, "Share a link"), "back to the receipt");
}

/// The share server for links made before: the ones whose id starts with "gone" were deleted on the
/// website.
struct LinkServer(std::sync::Mutex<Vec<String>>);

impl Http for LinkServer {
    fn send(&self, method: &str, url: &str, _headers: &[(&str, String)], _body: Option<Vec<u8>>) -> Result<Response, ()> {
        self.0.lock().unwrap().push(format!("{method} {url}"));
        let json = |status: u16, v: serde_json::Value| Response { status, headers: [("content-type".to_string(), "application/json".to_string())].into(), body: v.to_string().into_bytes() };
        Ok(if url.ends_with("/app/latest.json") {
            Response { status: 404, headers: Default::default(), body: vec![] }
        } else if url.contains("/api/reports/gone") {
            json(404, serde_json::json!({ "error": "not_found" }))
        } else {
            json(200, serde_json::json!({ "report": {}, "expiresAt": "2026-10-20T00:00:00Z" }))
        })
    }
}

#[test]
fn my_shared_links_can_check_which_still_work() {
    let dir = temp_dir();
    let saved = |id: &str| fettle_windows::store::SavedLink { id: id.into(), url: format!("https://fettle.page/r/{id}"), delete_token: "t".into(), expires_at: "2026-10-20T00:00:00Z".into() };
    FileStore::new(dir.clone()).save_links(&[saved("alive1234567890abcd"), saved("gone12345678901abcd")]);
    let server = Arc::new(LinkServer(Default::default()));
    let s = server.clone();
    let h = harness_with(Locale::En, dir.clone(), move |deps| deps.http = s);
    press(&h.app, "Shared links · 2");
    assert!(server.0.lock().unwrap().iter().all(|r| !r.contains("/api/reports/")), "nothing asked until the button");
    press(&h.app, "Check which still work");
    h.background.run_all(&h.app);
    assert!(has(&h.app, "1 link was deleted on the website, so it's gone from this list."));
    assert!(!has(&h.app, "https://fettle.page/r/gone12345678901abcd"));
    assert!(has(&h.app, "https://fettle.page/r/alive1234567890abcd"));
    assert_eq!(FileStore::new(dir).load_links(0).len(), 1, "remembered");
}
