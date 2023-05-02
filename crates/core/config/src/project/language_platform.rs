use moon_error::MoonError;
use moon_utils::regex::clean_id;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};
use strum::{Display, EnumIter, EnumString};

#[derive(Clone, Debug, Default, EnumIter, Eq, JsonSchema, PartialEq)]
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

impl<'de> Deserialize<'de> for ProjectLanguage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let lang = String::deserialize(deserializer)?.to_lowercase();

        Ok(ProjectLanguage::from_str(&lang).unwrap())
    }
}

impl Serialize for ProjectLanguage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl FromStr for ProjectLanguage {
    type Err = MoonError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_ref() {
            "bash" => ProjectLanguage::Bash,
            "batch" => ProjectLanguage::Batch,
            "go" => ProjectLanguage::Go,
            "javascript" => ProjectLanguage::JavaScript,
            "php" => ProjectLanguage::Php,
            "python" => ProjectLanguage::Python,
            "ruby" => ProjectLanguage::Ruby,
            "rust" => ProjectLanguage::Rust,
            "typescript" => ProjectLanguage::TypeScript,
            "unknown" => ProjectLanguage::Unknown,
            other => ProjectLanguage::Other(clean_id(other)),
        })
    }
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
    EnumString,
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

    #[strum(serialize = "rust")]
    Rust,

    #[strum(serialize = "system")]
    System,

    #[default]
    #[strum(serialize = "unknown")]
    Unknown,
}

impl PlatformType {
    pub fn is_system(&self) -> bool {
        matches!(self, PlatformType::System)
    }

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
            ProjectLanguage::Rust => PlatformType::Rust,
            // TODO: Move to these to their own platform once it's been implemented!
            ProjectLanguage::Go
            | ProjectLanguage::Php
            | ProjectLanguage::Python
            | ProjectLanguage::Ruby
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

    #[test]
    fn serializes_lang_to_string() {
        assert_eq!(
            serde_json::to_string(&ProjectLanguage::Go).unwrap(),
            "\"go\""
        );
        assert_eq!(
            serde_json::to_string(&ProjectLanguage::JavaScript).unwrap(),
            "\"javascript\""
        );
        assert_eq!(
            serde_json::to_string(&ProjectLanguage::Ruby).unwrap(),
            "\"ruby\""
        );
        assert_eq!(
            serde_json::to_string(&ProjectLanguage::Unknown).unwrap(),
            "\"unknown\""
        );
        assert_eq!(
            serde_json::to_string(&ProjectLanguage::Other("dotnet".into())).unwrap(),
            "\"dotnet\""
        );
    }

    #[test]
    fn deserializes_lang_to_enum() {
        assert_eq!(
            serde_json::from_str::<ProjectLanguage>("\"go\"").unwrap(),
            ProjectLanguage::Go,
        );
        assert_eq!(
            serde_json::from_str::<ProjectLanguage>("\"javascript\"").unwrap(),
            ProjectLanguage::JavaScript,
        );
        assert_eq!(
            serde_json::from_str::<ProjectLanguage>("\"ruby\"").unwrap(),
            ProjectLanguage::Ruby,
        );
        assert_eq!(
            serde_json::from_str::<ProjectLanguage>("\"unknown\"").unwrap(),
            ProjectLanguage::Unknown,
        );
        assert_eq!(
            serde_json::from_str::<ProjectLanguage>("\"dotnet\"").unwrap(),
            ProjectLanguage::Other("dotnet".into()),
        );
    }
}
