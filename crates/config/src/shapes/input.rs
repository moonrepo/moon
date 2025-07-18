use crate::portable_path::{FilePath, GlobPath, PortablePath, is_glob_like};
use crate::validate::validate_child_relative_path;
use crate::{config_struct, config_unit_enum, patterns};
use moon_common::Id;
use moon_common::path::standardize_separators;
use schematic::{Config, ConfigEnum, ParseError};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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

fn create_path_from_uri(uri: &Url) -> String {
    let path = uri.path();

    match uri.host_str() {
        // The first segment in a project relative is the host
        Some(host) => format!("{host}{}", if path == "/" { "" } else { path }),
        // While workspace relative does not have a host
        None => path.to_owned(),
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
    pub fn get_path(&self) -> &str {
        let path = self.file.as_str();

        if self.is_workspace_relative() {
            &path[1..]
        } else {
            path
        }
    }

    pub fn is_workspace_relative(&self) -> bool {
        self.file.as_str().starts_with('/')
    }

    pub fn from_uri(uri: Url) -> Result<Self, ParseError> {
        let mut input = Self {
            file: FilePath::parse(create_path_from_uri(&uri).as_str())?,
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
    pub fn get_path(&self) -> &str {
        let path = self.glob.as_str();

        if self.is_workspace_relative() {
            if self.is_negated() {
                &path[2..]
            } else {
                &path[1..]
            }
        } else {
            if self.is_negated() { &path[1..] } else { path }
        }
    }

    pub fn is_negated(&self) -> bool {
        self.glob.as_str().starts_with('!')
    }

    pub fn is_workspace_relative(&self) -> bool {
        let path = self.glob.as_str();

        path.starts_with('/') || path.starts_with("!/")
    }

    pub fn from_uri(uri: Url) -> Result<Self, ParseError> {
        let mut value = create_path_from_uri(&uri);

        // Fix invalid negated workspace paths
        if value.starts_with("/!") {
            value = format!("!/{}", &value[2..]);
        }

        let mut input = Self {
            glob: GlobPath::parse(&value)?,
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
    // FileGroup(FileGroupInput),
    // ManifestDeps(ManifestDepsInput),
    // ProjectSources(ProjectSourcesInput),
}

impl Input {
    pub fn create_uri(value: &str) -> Result<Url, ParseError> {
        // Always use forward slashes
        let mut value = standardize_separators(value);

        // Convert literal paths to a URI
        if !value.contains("://") {
            if is_glob_like(&value) {
                value = format!("glob://{value}");
            } else {
                value = format!("file://{value}");
            }
        }

        Url::parse(&value).map_err(map_parse_error)
    }

    pub fn parse(value: impl AsRef<str>) -> Result<Self, ParseError> {
        Self::from_str(value.as_ref())
    }

    pub fn is_glob(&self) -> bool {
        matches!(
            self,
            Self::EnvVarGlob(_) | Self::ProjectGlob(_) | Self::WorkspaceGlob(_)
        )
    }
}

impl FromStr for Input {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // Token function
        if value.starts_with('@') && patterns::TOKEN_FUNC_DISTINCT.is_match(value) {
            return Ok(Self::TokenFunc(value.to_owned()));
        }

        // Token/environment variable
        if let Some(var) = value.strip_prefix('$') {
            if patterns::ENV_VAR_DISTINCT.is_match(value) {
                return Ok(Self::EnvVar(var.to_owned()));
            } else if patterns::ENV_VAR_GLOB_DISTINCT.is_match(value) {
                return Ok(Self::EnvVarGlob(var.to_owned()));
            } else if patterns::TOKEN_VAR_DISTINCT.is_match(value) {
                return Ok(Self::TokenVar(value.to_owned()));
            }
        }

        // URI formats
        let uri = Self::create_uri(value)?;
        let is_workspace_relative = uri.host_str().is_none_or(|host| host == "!");

        match uri.scheme() {
            "file" => {
                let file = FileInput::from_uri(uri)?;

                validate_child_relative_path(file.get_path()).map_err(map_parse_error)?;

                return Ok(if is_workspace_relative {
                    Self::WorkspaceFile(file)
                } else {
                    Self::ProjectFile(file)
                });
            }
            "glob" => {
                let glob = GlobInput::from_uri(uri)?;

                validate_child_relative_path(glob.get_path()).map_err(map_parse_error)?;

                return Ok(if is_workspace_relative {
                    Self::WorkspaceGlob(glob)
                } else {
                    Self::ProjectGlob(glob)
                });
            }
            other => {
                return Err(ParseError::new(format!(
                    "input protocol `{other}://` is not supported"
                )));
            }
        }
    }
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
            // Input::FileGroup(input) => FileGroupInput::serialize(input, serializer),
            // Input::ManifestDeps(input) => ManifestDepsInput::serialize(input, serializer),
            Input::ProjectFile(input) => FileInput::serialize(input, serializer),
            Input::ProjectGlob(input) => GlobInput::serialize(input, serializer),
            // Input::ProjectSources(input) => ProjectSourcesInput::serialize(input, serializer),
            Input::WorkspaceFile(input) => FileInput::serialize(input, serializer),
            Input::WorkspaceGlob(input) => GlobInput::serialize(input, serializer),
        }
    }
}

// impl<'de> Deserialize<'de> for Input {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         deserializer.deserialize_any(visitor)
//     }
// }
