//! A saved scan: the raw sources, in the shape the scan's sources return them, with when and how it
//! ran. The test dumps are saved scans; `report` makes of one what `fettle-collect` would have made
//! of the same raw data, which is where the receipt corpus and the website's demo get their reports.

use crate::{Options, Raw, build_report};
use fettle_facts::Report;
use fettle_facts::redact::Redactor;
use serde::Deserialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Deserialize)]
pub struct Dump {
    pub collected_at_unix: u64,
    pub elevated: bool,
    pub sources: Raw,
}

impl Dump {
    pub fn from_json(text: &str) -> serde_json::Result<Dump> {
        serde_json::from_str(text)
    }

    pub fn collected_at(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(self.collected_at_unix)
    }

    /// The finished report, privacy pass included. A dump holds no names of its own, so the pass has
    /// none to add: it hides what it can find without them and gives startup items their readable names.
    pub fn report(&self) -> Report {
        build_report(&self.sources, self.collected_at(), self.elevated, &Redactor::default(), &Options::default().app_version)
    }
}
