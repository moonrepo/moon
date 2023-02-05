use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Display,
    EnumIter,
    Eq,
    JsonSchema,
    PartialEq,
    Serialize,
)]
#[serde(rename_all = "lowercase")]
pub enum ProjectLanguage {
    #[strum(serialize = "bash")]
    Bash,

    #[strum(serialize = "batch")]
    Batch,

    #[strum(serialize = "go")]
    Go,

    #[strum(serialize = "javascript")]
    JavaScript,

    #[strum(serialize = "php")]
    Php,

    #[strum(serialize = "python")]
    Python,

    #[strum(serialize = "ruby")]
    Ruby,

    #[strum(serialize = "rust")]
    Rust,

    #[strum(serialize = "typescript")]
    TypeScript,

    #[default]
    #[strum(serialize = "unknown")]
    Unknown,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Display,
    Eq,
    EnumIter,
    Hash,
    JsonSchema,
    PartialEq,
    Serialize,
)]
#[serde(rename_all = "lowercase")]
pub enum PlatformType {
    #[strum(serialize = "node")]
    Node,

    #[strum(serialize = "system")]
    System,

    #[default]
    #[strum(serialize = "unknown")]
    Unknown,
}

impl PlatformType {
    pub fn is_unknown(&self) -> bool {
        matches!(self, PlatformType::Unknown)
    }
}

impl From<ProjectLanguage> for PlatformType {
    fn from(language: ProjectLanguage) -> Self {
        match language {
            ProjectLanguage::Unknown => PlatformType::Unknown,
            ProjectLanguage::Bash | ProjectLanguage::Batch => PlatformType::System,
            ProjectLanguage::JavaScript | ProjectLanguage::TypeScript => PlatformType::Node,
            // TODO: Move to these to their own platform once it's been implemented!
            ProjectLanguage::Go
            | ProjectLanguage::Php
            | ProjectLanguage::Python
            | ProjectLanguage::Ruby
            | ProjectLanguage::Rust => PlatformType::System,
        }
    }
}
