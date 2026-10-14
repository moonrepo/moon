use super::portable_path::{FilePath, GlobPath, PortablePath, is_glob_like};
use super::*;
use crate::{
    DependencyScope, config_enum, config_struct, config_unit_enum, generate_io_file_methods,
    generate_io_glob_methods, patterns,
};
use moon_common::Id;
use moon_common::path::{
    RelativeFrom, WorkspaceRelativePathBuf, expand_to_workspace_relative, standardize_separators,
};
use schematic::{Config, ConfigEnum, ParseError, RegexSetting, Schema, SchemaBuilder, Schematic};
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

config_enum!(
    /// The string form of an input type.
    #[derive(Default)]
    #[serde(try_from = "String", into = "String")]
    pub enum InputString {
        #[default]
        None,
        EnvVar(String),
        EnvVarGlob(String),
        // Old
        TokenFunc(String),
        TokenVar(String),
    }
);

impl InputString {
    pub fn as_str(&self) -> &str {
        match self {
            Self::None => unreachable!(),
            Self::EnvVar(value)
            | Self::EnvVarGlob(value)
            | Self::TokenFunc(value)
            | Self::TokenVar(value) => value,
        }
    }
}

impl FromStr for InputString {
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

        Err(ParseError::new("invalid input string"))
    }
}

impl TryFrom<String> for InputString {
    type Error = ParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl Into<String> for InputString {
    fn into(self) -> String {
        match self {
            Self::None => unreachable!(),
            Self::EnvVar(value) | Self::EnvVarGlob(value) => format!("${value}"),
            Self::TokenFunc(value) | Self::TokenVar(value) => value,
        }
    }
}

impl Schematic for InputString {
    fn build_schema(mut schema: SchemaBuilder) -> Schema {
        schema.string_default()
    }
}

config_enum!(
    /// The different patterns a task input can be defined as.
    #[derive(Config)]
    #[serde(untagged)]
    pub enum Input {
        String(InputString),
        #[setting(nested)]
        FileGroup(FileGroupInput),
        #[setting(nested)]
        Project(ProjectInput),
        #[setting(nested)]
        File(FileInput),
        #[setting(nested)]
        Glob(GlobInput),
    }
);

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
            Self::String(value) => value.as_str(),
            Self::File(value) => value.file.as_str(),
            Self::FileGroup(value) => value.group.as_str(),
            Self::Glob(value) => value.glob.as_str(),
            Self::Project(value) => value.project.as_str(),
        }
    }

    pub fn is_glob(&self) -> bool {
        matches!(
            self,
            Self::String(InputString::EnvVarGlob(_)) | Self::Glob(_)
        )
    }
}

impl FromStr for Input {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // String formats
        if let Ok(inner) = InputString::from_str(value) {
            return Ok(Self::String(inner));
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
