//! What the rules produce: verdict lines that point into the copy catalogs.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A message parameter: TypeScript's `string | number`, with integers kept integral in JSON.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum Param {
    Int(i64),
    Float(f64),
    Str(String),
}

impl From<&str> for Param {
    fn from(s: &str) -> Self {
        Param::Str(s.to_string())
    }
}
impl From<String> for Param {
    fn from(s: String) -> Self {
        Param::Str(s)
    }
}
impl From<i64> for Param {
    fn from(n: i64) -> Self {
        Param::Int(n)
    }
}

/// A pointer into the copy catalogs; rendered per locale by `render`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MessageRef {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<BTreeMap<String, Param>>,
}

impl MessageRef {
    pub fn key(key: impl Into<String>) -> Self {
        MessageRef { key: key.into(), params: None }
    }

    pub fn with(key: impl Into<String>, params: &[(&str, Param)]) -> Self {
        MessageRef { key: key.into(), params: Some(params.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()) }
    }
}

/// Verdict levels. `Info` is for neutral notes that never affect the summary.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Fine,
    KeepAnEye,
    ActSoon,
    CouldntCheck,
    Info,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum SectionId {
    Storage,
    Battery,
    Memory,
    Windows,
    Startup,
    Graphics,
    System,
}

impl SectionId {
    pub fn key(&self) -> &'static str {
        match self {
            SectionId::Storage => "storage",
            SectionId::Battery => "battery",
            SectionId::Memory => "memory",
            SectionId::Windows => "windows",
            SectionId::Startup => "startup",
            SectionId::Graphics => "graphics",
            SectionId::System => "system",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Line {
    pub id: String,
    pub level: Level,
    pub headline: MessageRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<MessageRef>,
    /// Required for keep_an_eye and act_soon: every warning comes with a next step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<MessageRef>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SectionVerdict {
    pub section: SectionId,
    pub lines: Vec<Line>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Summary {
    /// summary.fine, summary.some, summary.attention or summary.partial.
    #[cfg_attr(feature = "schema", schemars(extend("enum" = ["summary.fine", "summary.some", "summary.attention", "summary.partial"])))]
    pub key: String,
    pub actions: Vec<MessageRef>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Receipt {
    pub summary: Summary,
    pub sections: Vec<SectionVerdict>,
    #[serde(rename = "engineVersion")]
    pub engine_version: String,
}
