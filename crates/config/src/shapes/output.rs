use super::portable_path::{FilePath, GlobPath, PortablePath, is_glob_like};
use super::*;
use crate::{config_struct, patterns};
use moon_common::path::{
    RelativeFrom, WorkspaceRelativePathBuf, expand_to_workspace_relative, standardize_separators,
};
use schematic::{Config, ParseError, Schema, SchemaBuilder, Schematic, schema::UnionType};
use serde::{Deserialize, Serialize, Serializer};
use std::str::FromStr;

config_struct!(
    /// A file path output.
    #[derive(Config)]
    pub struct FileOutput {
        pub file: FilePath,

        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub optional: Option<bool>,
    }
);

impl FileOutput {
    pub fn from_uri(uri: Uri) -> Result<Self, ParseError> {
        let mut output = Self {
            file: FilePath::parse(&uri.path)?,
            ..Default::default()
        };

        for (key, value) in uri.query {
            match key.as_str() {
                "optional" => {
                    output.optional = Some(parse_bool_field(&key, &value)?);
                }
                _ => {
                    return Err(ParseError::new(format!("unknown field `{key}`")));
                }
            };
        }

        Ok(output)
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

config_struct!(
    /// A glob path output.
    #[derive(Config)]
    pub struct GlobOutput {
        pub glob: GlobPath,
    }
);

impl GlobOutput {
    pub fn from_uri(uri: Uri) -> Result<Self, ParseError> {
        let output = Self {
            glob: GlobPath::parse(uri.path.replace("__QM__", "?"))?,
        };

        if let Some((key, _)) = uri.query.into_iter().next() {
            return Err(ParseError::new(format!("unknown field `{key}`")));
        }

        Ok(output)
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

/// The different patterns a task output can be defined as.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(try_from = "OutputBase")]
pub enum Output {
    File(FileOutput),
    Glob(GlobOutput),
    // Old
    TokenFunc(String),
    TokenVar(String),
}

impl Output {
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
            Self::TokenFunc(value) | Self::TokenVar(value) => value,
            Self::File(value) => value.file.as_str(),
            Self::Glob(value) => value.glob.as_str(),
        }
    }

    pub fn is_glob(&self) -> bool {
        matches!(self, Self::Glob(_))
    }
}

impl FromStr for Output {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // Token function
        if value.starts_with('@') && patterns::TOKEN_FUNC_DISTINCT.is_match(value) {
            return Ok(Self::TokenFunc(value.to_owned()));
        }

        // Token/environment variable
        if value.starts_with('$') {
            if patterns::ENV_VAR_DISTINCT.is_match(value) {
                return Err(ParseError::new(
                    "environment variable is not supported by itself",
                ));
            } else if patterns::ENV_VAR_GLOB_DISTINCT.is_match(value) {
                return Err(ParseError::new(
                    "environment variable globs are not supported",
                ));
            } else if patterns::TOKEN_VAR_DISTINCT.is_match(value) {
                return Ok(Self::TokenVar(value.to_owned()));
            }
        }

        // URI formats
        let uri = Self::create_uri(value)?;

        match uri.scheme.as_str() {
            "file" => Ok(Self::File(FileOutput::from_uri(uri)?)),
            "glob" => Ok(Self::Glob(GlobOutput::from_uri(uri)?)),
            other => Err(ParseError::new(format!(
                "output protocol `{other}://` is not supported"
            ))),
        }
    }
}

impl TryFrom<OutputBase> for Output {
    type Error = ParseError;

    fn try_from(base: OutputBase) -> Result<Self, Self::Error> {
        match base {
            OutputBase::Raw(output) => Self::parse(output),
            OutputBase::File(output) => Ok(Self::File(output)),
            OutputBase::Glob(output) => Ok(Self::Glob(output)),
        }
    }
}

impl Schematic for Output {
    fn schema_name() -> Option<String> {
        Some("Output".into())
    }

    fn build_schema(mut schema: SchemaBuilder) -> Schema {
        schema.union(UnionType::new_any([
            schema.infer::<String>(),
            schema.infer::<FileOutput>(),
            schema.infer::<GlobOutput>(),
        ]))
    }
}

impl Serialize for Output {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Output::TokenFunc(token) | Output::TokenVar(token) => serializer.serialize_str(token),
            Output::File(output) => FileOutput::serialize(output, serializer),
            Output::Glob(output) => GlobOutput::serialize(output, serializer),
        }
    }
}

#[derive(Deserialize)]
#[serde(
    untagged,
    expecting = "expected a file path, glob pattern, URI string, or object"
)]
enum OutputBase {
    Raw(String),
    // From most complex to least
    File(FileOutput),
    Glob(GlobOutput),
}
