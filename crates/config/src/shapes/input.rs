use super::portable_path::{FilePath, GlobPath, PortablePath, is_glob_like};
use super::{Uri, is_false};
use crate::{config_struct, config_unit_enum, patterns};
use moon_common::Id;
use moon_common::path::{
    RelativeFrom, WorkspaceRelativePathBuf, expand_to_workspace_relative, standardize_separators,
};
use schematic::{
    Config, ConfigEnum, ParseError, RegexSetting, Schema, SchemaBuilder, Schematic,
    schema::UnionType,
};
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

fn map_parse_error<T: fmt::Display>(error: T) -> ParseError {
    ParseError::new(error.to_string())
}

fn default_true() -> bool {
    true
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

        #[serde(
            default,
            alias = "match",
            alias = "matches",
            skip_serializing_if = "Option::is_none"
        )]
        pub content: Option<RegexSetting>,

        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub optional: Option<bool>,
    }
);

impl FileInput {
    pub fn from_uri(uri: Uri) -> Result<Self, ParseError> {
        let mut input = Self {
            file: FilePath::parse(&uri.path)?,
            ..Default::default()
        };

        for (key, value) in uri.query {
            match key.as_str() {
                "content" | "match" | "matches" => {
                    if !value.is_empty() {
                        input.content = Some(RegexSetting::new(value).map_err(map_parse_error)?);
                    }
                }
                "optional" => {
                    input.optional = Some(parse_bool_field(&key, &value)?);
                }
                _ => {
                    return Err(ParseError::new(format!("unknown field `{key}`")));
                }
            };
        }

        Ok(input)
    }

    pub fn get_path(&self) -> String {
        let path = self.file.as_str();

        if self.is_workspace_relative() {
            path[1..].into()
        } else {
            path.into()
        }
    }

    pub fn is_workspace_relative(&self) -> bool {
        self.file.as_str().starts_with('/')
    }

    pub fn to_workspace_relative(
        &self,
        project_source: impl AsRef<str>,
    ) -> WorkspaceRelativePathBuf {
        expand_to_workspace_relative(
            if self.is_workspace_relative() {
                RelativeFrom::Workspace
            } else {
                RelativeFrom::Project(project_source.as_ref())
            },
            self.get_path(),
        )
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

        #[serde(default, alias = "as")]
        pub format: FileGroupInputFormat,
    }
);

impl FileGroupInput {
    pub fn from_uri(uri: Uri) -> Result<Self, ParseError> {
        let mut input = Self {
            group: if uri.path.is_empty() {
                return Err(ParseError::new("a file group identifier is required"));
            } else {
                Id::new(&uri.path).map_err(map_parse_error)?
            },
            ..Default::default()
        };

        for (key, value) in uri.query {
            match key.as_str() {
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

        #[serde(default = "default_true", skip_serializing_if = "is_false")]
        #[setting(default = true)]
        pub cache: bool,
    }
);

impl GlobInput {
    pub fn from_uri(uri: Uri) -> Result<Self, ParseError> {
        let mut input = Self {
            glob: GlobPath::parse(uri.path.replace("__QM__", "?"))?,
            ..Default::default()
        };

        for (key, value) in uri.query {
            match key.as_str() {
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

    pub fn get_path(&self) -> String {
        let path = self.glob.as_str();

        if self.is_workspace_relative() {
            if self.is_negated() {
                format!("!{}", &path[2..])
            } else {
                path[1..].into()
            }
        } else {
            path.into()
        }
    }

    pub fn is_negated(&self) -> bool {
        self.glob.as_str().starts_with('!')
    }

    pub fn is_workspace_relative(&self) -> bool {
        let path = self.glob.as_str();

        path.starts_with('/') || path.starts_with("!/")
    }

    pub fn to_workspace_relative(
        &self,
        project_source: impl AsRef<str>,
    ) -> WorkspaceRelativePathBuf {
        expand_to_workspace_relative(
            if self.is_workspace_relative() {
                RelativeFrom::Workspace
            } else {
                RelativeFrom::Project(project_source.as_ref())
            },
            self.get_path(),
        )
    }
}

config_struct!(
    /// A manifest file's dependencies input.
    #[derive(Config)]
    pub struct ManifestDepsInput {
        pub manifest: Id, // toolchain

        #[serde(
            default,
            alias = "dep",
            alias = "dependencies",
            skip_serializing_if = "Vec::is_empty"
        )]
        pub deps: Vec<String>,
    }
);

impl ManifestDepsInput {
    pub fn from_uri(uri: Uri) -> Result<Self, ParseError> {
        let mut input = Self {
            manifest: if uri.path.is_empty() {
                return Err(ParseError::new("a toolchain identifier is required"));
            } else {
                Id::new(&uri.path).map_err(map_parse_error)?
            },
            ..Default::default()
        };

        for (key, value) in uri.query {
            match key.as_str() {
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
    /// An external project input.
    #[derive(Config)]
    pub struct ProjectInput {
        // This is not an `Id` as we need to support "^".
        pub project: String,

        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub filter: Vec<String>,

        #[serde(default, alias = "fileGroup", skip_serializing_if = "Option::is_none")]
        pub group: Option<Id>,
    }
);

impl ProjectInput {
    pub fn from_uri(uri: Uri) -> Result<Self, ParseError> {
        let mut input = Self {
            project: if uri.path.is_empty() {
                return Err(ParseError::new("a project identifier is required"));
            } else if uri.path == "^" {
                uri.path
            } else {
                Id::new(&uri.path).map_err(map_parse_error)?.to_string()
            },
            ..Default::default()
        };

        for (key, value) in uri.query {
            match key.as_str() {
                "filter" => {
                    if !value.is_empty() {
                        input.filter.push(value);
                    }
                }
                "fileGroup" | "group" => {
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

/// The different patterns a task input can be defined as.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(try_from = "InputBase")]
pub enum Input {
    EnvVar(String),
    EnvVarGlob(String),
    File(FileInput),
    FileGroup(FileGroupInput),
    Glob(GlobInput),
    Project(ProjectInput),
    // Old
    TokenFunc(String),
    TokenVar(String),
    // New
    // ManifestDeps(ManifestDepsInput),
}

impl Input {
    pub fn create_uri(value: &str) -> Result<Uri, ParseError> {
        // Always use forward slashes
        let mut value = standardize_separators(value);

        // Convert literal paths to a URI
        if !value.contains("://") {
            if is_glob_like(&value) {
                value = format!("glob://{}", value.replace("?", "__QM__"));
            } else {
                value = format!("file://{value}");
            }
        }

        Uri::parse(&value)
    }

    pub fn parse(value: impl AsRef<str>) -> Result<Self, ParseError> {
        Self::from_str(value.as_ref())
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::EnvVar(value)
            | Self::EnvVarGlob(value)
            | Self::TokenFunc(value)
            | Self::TokenVar(value) => value,
            Self::File(value) => value.file.as_str(),
            Self::FileGroup(value) => value.group.as_str(),
            Self::Glob(value) => value.glob.as_str(),
            Self::Project(value) => value.project.as_str(),
        }
    }

    pub fn is_glob(&self) -> bool {
        matches!(self, Self::EnvVarGlob(_) | Self::Glob(_))
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

        match uri.scheme.as_str() {
            "file" => Ok(Self::File(FileInput::from_uri(uri)?)),
            "glob" => Ok(Self::Glob(GlobInput::from_uri(uri)?)),
            "group" | "filegroup" | "fileGroup" => {
                Ok(Self::FileGroup(FileGroupInput::from_uri(uri)?))
            }
            "project" => Ok(Self::Project(ProjectInput::from_uri(uri)?)),
            other => Err(ParseError::new(format!(
                "input protocol `{other}://` is not supported"
            ))),
        }
    }
}

impl TryFrom<InputBase> for Input {
    type Error = ParseError;

    fn try_from(base: InputBase) -> Result<Self, Self::Error> {
        match base {
            InputBase::Raw(input) => Self::parse(input),
            InputBase::File(input) => Ok(Self::File(input)),
            InputBase::FileGroup(input) => Ok(Self::FileGroup(input)),
            InputBase::Glob(input) => Ok(Self::Glob(input)),
            InputBase::Project(input) => Ok(Self::Project(input)),
        }
    }
}

impl Schematic for Input {
    fn schema_name() -> Option<String> {
        Some("Input".into())
    }

    fn build_schema(mut schema: SchemaBuilder) -> Schema {
        schema.union(UnionType::new_any([
            schema.infer::<String>(),
            schema.infer::<FileInput>(),
            schema.infer::<FileGroupInput>(),
            schema.infer::<GlobInput>(),
            schema.infer::<ProjectInput>(),
        ]))
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
            Input::File(input) => FileInput::serialize(input, serializer),
            Input::FileGroup(input) => FileGroupInput::serialize(input, serializer),
            // Input::ManifestDeps(input) => ManifestDepsInput::serialize(input, serializer),
            Input::Glob(input) => GlobInput::serialize(input, serializer),
            Input::Project(input) => ProjectInput::serialize(input, serializer),
        }
    }
}

#[derive(Deserialize)]
#[serde(
    untagged,
    expecting = "expected a file path, glob pattern, URI string, or object"
)]
enum InputBase {
    Raw(String),
    File(FileInput),
    FileGroup(FileGroupInput),
    Glob(GlobInput),
    Project(ProjectInput),
}
