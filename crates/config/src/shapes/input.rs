use super::portable_path::{FilePath, GlobPath, PortablePath, is_glob_like};
use super::*;
use crate::{
    DependencyScope, config_struct, config_unit_enum, generate_io_file_methods,
    generate_io_glob_methods, patterns,
};
use moon_common::Id;
use moon_common::path::{
    RelativeFrom, WorkspaceRelativePathBuf, expand_to_workspace_relative, standardize_separators,
};
use schematic::{
    Config, ConfigEnum, ParseError, RegexSetting, Schema, SchemaBuilder, Schematic,
    schema::UnionType,
};
use serde::{Deserialize, Serialize, Serializer};
use std::str::FromStr;

config_struct!(
    /// A file path input.
    #[derive(Config)]
    pub struct FileInput {
        /// The literal file path.
        #[setting(required)]
        pub file: FilePath,

        /// Regex pattern to match the file's contents against
        /// when determining affected status.
        #[serde(
            default,
            alias = "match",
            alias = "matches",
            skip_serializing_if = "Option::is_none"
        )]
        pub content: Option<RegexSetting>,

        /// Mark the file as optional instead of logging a warning
        /// when hashing a task.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub optional: Option<bool>,

        /// Only use the file to determine affected status, and exclude
        /// it from the task's hash and affected files.
        #[serde(default, skip_serializing_if = "is_false")]
        pub affected_only: bool,
    }
);

generate_io_file_methods!(FileInput);

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
                "affectedOnly" | "affectedonly" => {
                    input.affected_only = parse_bool_field(&key, &value)?;
                }
                _ => {
                    return Err(ParseError::new(format!("unknown file field `{key}`")));
                }
            };
        }

        Ok(input)
    }
}

config_unit_enum!(
    /// Available formats to resolve the file group into.
    #[derive(ConfigEnum, Default)]
    pub enum FileGroupInputFormat {
        /// Return the group as-is.
        #[default]
        Static,
        /// Return only directories.
        Dirs,
        /// Return only environment variables.
        Envs,
        /// Return only files.
        Files,
        /// Return only globs.
        Globs,
        /// Return the lowest common root of all paths.
        Root,
    }
);

config_struct!(
    /// A file group input.
    #[derive(Config)]
    pub struct FileGroupInput {
        /// The file group identifier.
        #[setting(required)]
        pub group: Id,

        /// Format to resolve the file group into.
        // Marks the setting as optional in schemas
        #[serde(default, alias = "as")]
        pub format: FileGroupInputFormat,

        /// Only use the files and globs of the group to determine affected
        /// status, and exclude them from the task's hash and affected files.
        #[serde(default, skip_serializing_if = "is_false")]
        pub affected_only: bool,
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
                "affectedOnly" | "affectedonly" => {
                    input.affected_only = parse_bool_field(&key, &value)?;
                }
                _ => {
                    return Err(ParseError::new(format!("unknown file group field `{key}`")));
                }
            };
        }

        Ok(input)
    }
}

config_struct!(
    /// A glob pattern input.
    #[derive(Config)]
    pub struct GlobInput {
        /// The glob pattern.
        #[setting(required)]
        pub glob: GlobPath,

        /// Cache the glob walking result for increased performance.
        #[setting(default = true)]
        pub cache: bool,

        /// Only use the glob to determine affected status, and exclude
        /// its files from the task's hash and affected files.
        #[serde(default, skip_serializing_if = "is_false")]
        pub affected_only: bool,
    }
);

generate_io_glob_methods!(GlobInput);

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
                "affectedOnly" | "affectedonly" => {
                    input.affected_only = parse_bool_field(&key, &value)?;
                }
                _ => {
                    return Err(ParseError::new(format!("unknown glob field `{key}`")));
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
        /// The toolchain identifier.
        #[setting(required)]
        pub manifest: Id,

        /// List of dependencies to compare against when
        /// determining affected status.
        // Marks the setting as optional in schemas
        #[serde(default, alias = "dep", alias = "dependencies")]
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
                    return Err(ParseError::new(format!("unknown manifest field `{key}`")));
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
        // Note: This is not an `Id` as we need to support `^` scopes!
        /// The external project identifier.
        #[setting(required)]
        pub project: String,

        /// A list of globs, relative from the project's root,
        /// in which to determine affected status.
        // Marks the setting as optional in schemas
        #[serde(default)]
        pub filter: Vec<String>,

        /// A file group identifier within the project in which
        /// to determine affected status.
        #[serde(default, alias = "fileGroup", skip_serializing_if = "Option::is_none")]
        pub group: Option<Id>,

        /// Only use the project's files to determine affected status, and
        /// exclude them from the task's hash and affected files.
        #[serde(default, skip_serializing_if = "is_false")]
        pub affected_only: bool,
    }
);

impl ProjectInput {
    pub fn from_uri(uri: Uri) -> Result<Self, ParseError> {
        let mut input = Self {
            project: if uri.path.is_empty() {
                return Err(ParseError::new("a project identifier is required"));
            } else if uri.path.starts_with('^') {
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
                "fileGroup" | "filegroup" | "group" => {
                    if !value.is_empty() {
                        input.group = Some(Id::new(&value).map_err(map_parse_error)?);
                    }
                }
                "affectedOnly" | "affectedonly" => {
                    input.affected_only = parse_bool_field(&key, &value)?;
                }
                _ => {
                    return Err(ParseError::new(format!("unknown project field `{key}`")));
                }
            };
        }

        Ok(input)
    }

    pub fn is_all_deps(&self) -> bool {
        self.project == "^"
    }

    pub fn get_deps_scope(&self) -> Option<DependencyScope> {
        match self.project.as_str() {
            "^dev" | "^development" => Some(DependencyScope::Development),
            "^prod" | "^production" => Some(DependencyScope::Production),
            "^peer" => Some(DependencyScope::Peer),
            "^build" => Some(DependencyScope::Build),
            _ => None,
        }
    }
}

/// The different patterns a task input can be defined as.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(try_from = "PartialInputShape")]
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

// Only the partial is used, which schematic deserializes by attempting
// each variant in order, and reports every error when none match. Partial
// fields are all optional, so a variant matches any object without unknown
// fields, and required fields are only checked when finalized. Because of
// this, `FileGroup` must come before `Project`, as both have a `group` field.
#[allow(dead_code)]
#[derive(Config)]
#[serde(untagged)]
enum InputShape {
    String(String),
    #[setting(nested)]
    FileGroup(FileGroupInput),
    #[setting(nested)]
    Project(ProjectInput),
    #[setting(nested)]
    File(FileInput),
    #[setting(nested)]
    Glob(GlobInput),
}

impl TryFrom<PartialInputShape> for Input {
    type Error = ParseError;

    fn try_from(base: PartialInputShape) -> Result<Self, Self::Error> {
        Ok(match base {
            PartialInputShape::String(input) => Self::parse(input)?,
            PartialInputShape::File(input) => Self::File(finalize_partial(input)?),
            PartialInputShape::FileGroup(input) => Self::FileGroup(finalize_partial(input)?),
            PartialInputShape::Glob(input) => Self::Glob(finalize_partial(input)?),
            PartialInputShape::Project(input) => Self::Project(finalize_partial(input)?),
        })
    }
}
