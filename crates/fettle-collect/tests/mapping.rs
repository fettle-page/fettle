//! What the mapping makes of each tracked dump, pinned: the facts in `tests/fixtures`, and the
//! reports in the receipt corpus (crates/fettle-receipt/tests/parity/cases), which the verdict and
//! receipt tests read. After a deliberate change to the mapping, `npm run parity:goldens` rewrites
//! both (it runs this with FETTLE_REGEN_GOLDENS=1), and the diff shows what changed.

use fettle_collect::dump::Dump;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn regenerating() -> bool {
    std::env::var_os("FETTLE_REGEN_GOLDENS").is_some()
}

/// Every `.json` file in a folder, by name.
fn json_files(dir: PathBuf) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "json")).collect();
    files.sort();
    files
}

fn stem(path: &Path) -> String {
    path.file_stem().unwrap().to_string_lossy().into_owned()
}

/// The tracked dumps: made-up scans of a desktop and a Windows 10 laptop (each as a user and as an
/// administrator), and one with odd values.
fn dumps() -> Vec<PathBuf> {
    json_files(root().join("tests/dumps"))
}

fn load(path: &Path) -> Dump {
    Dump::from_json(&fs::read_to_string(path).unwrap()).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn each_dump_maps_to_its_pinned_facts() {
    let mut checked = 0;
    for path in dumps() {
        let name = stem(&path);
        let dump = load(&path);
        let got = serde_json::to_value(fettle_collect::map::facts(&dump.sources, dump.collected_at())).unwrap();
        let file = root().join(format!("tests/fixtures/{name}.facts.json"));
        if regenerating() {
            fs::write(&file, format!("{}\n", serde_json::to_string_pretty(&got).unwrap())).unwrap();
        }
        let want: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        for (section, facts) in got.as_object().unwrap() {
            assert_eq!(facts, &want[section], "{name}: {section} (rewrite with npm run parity:goldens)");
        }
        assert_eq!(got, want, "{name}");
        checked += 1;
    }
    assert_eq!(checked, 5);
}

/// The receipt corpus's dump cases hold the report this crate makes of each dump, envelope and
/// privacy pass included: what the app would have shown on that PC.
#[test]
fn the_receipt_corpus_holds_the_report_made_from_each_dump() {
    let cases = root().join("../fettle-receipt/tests/parity/cases");
    let mut checked = 0;
    for path in dumps() {
        let name = format!("dump-{}", stem(&path));
        let got = serde_json::to_value(load(&path).report()).unwrap();
        let case: Value = serde_json::from_str(&fs::read_to_string(cases.join(format!("{name}.json"))).expect("a corpus case for each dump")).unwrap();
        assert_eq!(got, case["report"], "{name} (rewrite with npm run parity:goldens)");
        checked += 1;
    }
    assert_eq!(checked, 5);
}
