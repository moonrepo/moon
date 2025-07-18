use crate::portable_path::{FilePath, GlobPath};
use crate::{PortablePath, config_struct, config_unit_enum};
use moon_common::Id;
use schematic::{Config, ConfigEnum, ParseError};
use serde::{Serialize, Serializer};
use std::fmt::Display;
use std::str::FromStr;
use url::Url;

fn map_parse_error<T: Display>(error: T) -> ParseError {
    ParseError::new(error.to_string())
}

fn parse_bool_field(key: &str, value: &str) -> Result<bool, ParseError> {
    if value.is_empty() || value == "true" {
        Ok(true)
    } else if value == "false" {
        Ok(false)
    } else {
        Err(ParseError::new(format!("unsupported value for `{key}`")))
    }
}

config_struct!(
    /// A file path input.
    #[derive(Config)]
    pub struct FileInput {
        pub file: FilePath,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub matches: Option<String>,

        pub optional: bool,
    }
);

impl FileInput {
    pub fn from_uri(uri: Url) -> Result<Self, ParseError> {
        let mut input = Self {
            file: FilePath::parse(
                match uri.host_str() {
                    // The first segment in a project relative is the host
                    Some(host) => format!("{host}{}", uri.path()),
                    // While workspace relative does not have a host
                    None => uri.path().to_owned(),
                }
                .as_str(),
            )?,
            ..Default::default()
        };

        for (key, value) in uri.query_pairs() {
            match &*key {
                "match" | "matches" => {
                    if !value.is_empty() {
                        input.matches = Some(value.to_string());
                    }
                }
                "optional" => {
                    input.optional = parse_bool_field(&key, &value)?;
                }
                _ => {
                    return Err(ParseError::new(format!("unknown field `{key}`")));
                }
            };
        }

        Ok(input)
    }
}

config_unit_enum!(
    /// Format to resolve the file group into.
    #[derive(ConfigEnum)]
    pub enum FileGroupInputFormat {
        #[default]
        Static,
        Dirs,
        Envs,
        Files,
        Globs,
        Root,
    }
);

config_struct!(
    /// A file group input.
    #[derive(Config)]
    pub struct FileGroupInput {
        pub group: Id,
        pub format: FileGroupInputFormat,
    }
);

impl FileGroupInput {
    pub fn from_uri(uri: Url) -> Result<Self, ParseError> {
        let mut input = Self {
            group: match uri.host_str() {
                Some(host) => Id::new(host).map_err(map_parse_error)?,
                None => return Err(ParseError::new("a file group identifier is required")),
            },
            ..Default::default()
        };

        for (key, value) in uri.query_pairs() {
            match &*key {
                "as" | "format" => {
                    input.format =
                        FileGroupInputFormat::from_str(&value).map_err(map_parse_error)?
                }
                _ => {
                    return Err(ParseError::new(format!("unknown field `{key}`")));
                }
            };
        }

        Ok(input)
    }
}

config_struct!(
    /// A glob path input.
    #[derive(Config)]
    pub struct GlobInput {
        pub glob: GlobPath,

        #[setting(default = true)]
        pub cache: bool,
    }
);

impl GlobInput {
    pub fn from_uri(uri: Url) -> Result<Self, ParseError> {
        let mut input = Self {
            glob: GlobPath::parse(
                match uri.host_str() {
                    // The first segment in a project relative is the host
                    Some(host) => format!("{host}{}", uri.path()),
                    // While workspace relative does not have a host
                    None => uri.path().to_owned(),
                }
                .as_str(),
            )?,
            ..Default::default()
        };

        for (key, value) in uri.query_pairs() {
            match &*key {
                "cache" => {
                    input.cache = parse_bool_field(&key, &value)?;
                }
                _ => {
                    return Err(ParseError::new(format!("unknown field `{key}`")));
                }
            };
        }

        Ok(input)
    }
}

config_struct!(
    /// A manifest file's dependencies input.
    #[derive(Config)]
    pub struct ManifestDepsInput {
        pub manifest: Id, // toolchain

        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub deps: Vec<String>,
    }
);

impl ManifestDepsInput {
    pub fn from_uri(uri: Url) -> Result<Self, ParseError> {
        let mut input = Self {
            manifest: match uri.host_str() {
                Some(host) => Id::new(host).map_err(map_parse_error)?,
                None => return Err(ParseError::new("a toolchain identifier is required")),
            },
            ..Default::default()
        };

        for (key, value) in uri.query_pairs() {
            match &*key {
                "dep" | "deps" | "dependencies" => {
                    for val in value.split(',') {
                        if !val.is_empty() {
                            input.deps.push(val.trim().to_owned());
                        }
                    }
                }
                _ => {
                    return Err(ParseError::new(format!("unknown field `{key}`")));
                }
            };
        }

        Ok(input)
    }
}

config_struct!(
    /// An external project's sources input.
    #[derive(Config)]
    pub struct ProjectSourcesInput {
        pub project: Id,

        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub filter: Vec<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub group: Option<Id>,
    }
);

impl ProjectSourcesInput {
    pub fn from_uri(uri: Url) -> Result<Self, ParseError> {
        let mut input = Self {
            project: match uri.host_str() {
                Some(host) => Id::new(host).map_err(map_parse_error)?,
                None => return Err(ParseError::new("a project identifier is required")),
            },
            ..Default::default()
        };

        for (key, value) in uri.query_pairs() {
            match &*key {
                "filter" => {
                    if !value.is_empty() {
                        input.filter.push(value.to_string());
                    }
                }
                "fileGroup" | "file-group" | "group" => {
                    if !value.is_empty() {
                        input.group = Some(Id::new(&value).map_err(map_parse_error)?);
                    }
                }
                _ => {
                    return Err(ParseError::new(format!("unknown field `{key}`")));
                }
            };
        }

        Ok(input)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Input {
    EnvVar(String),
    EnvVarGlob(String),
    ProjectFile(FileInput),
    ProjectGlob(GlobInput),
    WorkspaceFile(FileInput),
    WorkspaceGlob(GlobInput),
    // Old
    TokenFunc(String),
    TokenVar(String),
    // New
    FileGroup(FileGroupInput),
    ManifestDeps(ManifestDepsInput),
    ProjectSources(ProjectSourcesInput),
}

impl Serialize for Input {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Input::EnvVar(var) | Input::EnvVarGlob(var) => {
                serializer.serialize_str(format!("${var}").as_str())
            }
            Input::TokenFunc(token) | Input::TokenVar(token) => serializer.serialize_str(token),
            Input::FileGroup(input) => FileGroupInput::serialize(input, serializer),
            Input::ManifestDeps(input) => ManifestDepsInput::serialize(input, serializer),
            Input::ProjectFile(input) => FileInput::serialize(input, serializer),
            Input::ProjectGlob(input) => GlobInput::serialize(input, serializer),
            Input::ProjectSources(input) => ProjectSourcesInput::serialize(input, serializer),
            Input::WorkspaceFile(input) => FileInput::serialize(input, serializer),
            Input::WorkspaceGlob(input) => GlobInput::serialize(input, serializer),
        }
    }
}
