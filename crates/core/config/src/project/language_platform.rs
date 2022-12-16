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

    #[strum(serialize = "javascript")]
    JavaScript,

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

impl From<ProjectLanguage> for PlatformType {
    fn from(language: ProjectLanguage) -> Self {
        match language {
            ProjectLanguage::Bash | ProjectLanguage::Batch => PlatformType::System,
            ProjectLanguage::JavaScript | ProjectLanguage::TypeScript => PlatformType::Node,
            ProjectLanguage::Unknown => PlatformType::Unknown,
        }
    }
}
