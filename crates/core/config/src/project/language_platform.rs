use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use strum::{Display, EnumIter};

#[derive(Clone, Debug, Default, Deserialize, EnumIter, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectLanguage {
    Bash,
    Batch,
    Go,
    JavaScript,
    Php,
    Python,
    Ruby,
    Rust,
    TypeScript,

    // Not explicitly set or detected
    #[default]
    Unknown,

    // An unsupported language
    Other(String),
}

impl fmt::Display for ProjectLanguage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ProjectLanguage::Bash => "bash",
                ProjectLanguage::Batch => "batch",
                ProjectLanguage::Go => "go",
                ProjectLanguage::JavaScript => "javascript",
                ProjectLanguage::Php => "php",
                ProjectLanguage::Python => "python",
                ProjectLanguage::Ruby => "ruby",
                ProjectLanguage::Rust => "rust",
                ProjectLanguage::TypeScript => "typescript",
                ProjectLanguage::Unknown => "unknown",
                ProjectLanguage::Other(lang) => lang,
            }
        )
    }
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
    #[strum(serialize = "deno")]
    Deno,

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
            // Deno and Bun are not covered here!
            ProjectLanguage::JavaScript | ProjectLanguage::TypeScript => PlatformType::Node,
            // TODO: Move to these to their own platform once it's been implemented!
            ProjectLanguage::Go
            | ProjectLanguage::Php
            | ProjectLanguage::Python
            | ProjectLanguage::Ruby
            | ProjectLanguage::Rust
            | ProjectLanguage::Other(_) => PlatformType::System,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_lang_to_string() {
        assert_eq!(ProjectLanguage::Go.to_string(), "go");
        assert_eq!(ProjectLanguage::JavaScript.to_string(), "javascript");
        assert_eq!(ProjectLanguage::Ruby.to_string(), "ruby");
        assert_eq!(ProjectLanguage::Unknown.to_string(), "unknown");
        assert_eq!(
            ProjectLanguage::Other("dotnet".into()).to_string(),
            "dotnet"
        );
    }
}
