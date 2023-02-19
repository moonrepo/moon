use crate::validators::{is_default, is_default_true, validate_semver_version};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

fn validate_node_version(value: &str) -> Result<(), ValidationError> {
    validate_semver_version("node.version", value)
}

fn validate_npm_version(value: &str) -> Result<(), ValidationError> {
    validate_semver_version("node.npm.version", value)
}

fn validate_pnpm_version(value: &str) -> Result<(), ValidationError> {
    validate_semver_version("node.pnpm.version", value)
}

fn validate_yarn_version(value: &str) -> Result<(), ValidationError> {
    validate_semver_version("node.yarn.version", value)
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NodeProjectAliasFormat {
    NameAndScope, // @scope/name
    NameOnly,     // name
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NodeVersionFormat {
    File,         // file:..
    Link,         // link:..
    Star,         // *
    Version,      // 0.0.0
    VersionCaret, // ^0.0.0
    VersionTilde, // ~0.0.0
    #[default]
    Workspace, // workspace:*
    WorkspaceCaret, // workspace:^
    WorkspaceTilde, // workspace:~
}

impl NodeVersionFormat {
    pub fn get_prefix(&self) -> String {
        match self {
            NodeVersionFormat::File => String::from("file:"),
            NodeVersionFormat::Link => String::from("link:"),
            NodeVersionFormat::Star => String::from("*"),
            NodeVersionFormat::Version => String::from(""),
            NodeVersionFormat::VersionCaret => String::from("^"),
            NodeVersionFormat::VersionTilde => String::from("~"),
            NodeVersionFormat::Workspace => String::from("workspace:*"),
            NodeVersionFormat::WorkspaceCaret => String::from("workspace:^"),
            NodeVersionFormat::WorkspaceTilde => String::from("workspace:~"),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NodePackageManager {
    #[default]
    Npm,
    Pnpm,
    Yarn,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeVersionManager {
    Nodenv,
    Nvm,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
pub struct NpmConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_npm_version")]
    pub version: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
pub struct PnpmConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_pnpm_version")]
    pub version: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
pub struct YarnConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_yarn_version")]
    pub version: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct NodeConfig {
    #[serde(skip_serializing_if = "is_default_true")]
    pub add_engines_constraint: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias_package_names: Option<NodeProjectAliasFormat>,

    #[serde(skip_serializing_if = "is_default")]
    pub bin_exec_args: Vec<String>,

    #[serde(skip_serializing_if = "is_default_true")]
    pub dedupe_on_lockfile_change: bool,

    #[serde(skip_serializing_if = "is_default")]
    pub dependency_version_format: NodeVersionFormat,

    #[serde(skip_serializing_if = "is_default")]
    pub infer_tasks_from_scripts: bool,

    #[serde(skip_serializing_if = "is_default")]
    #[validate]
    pub npm: NpmConfig,

    #[serde(skip_serializing_if = "is_default")]
    pub package_manager: NodePackageManager,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate]
    pub pnpm: Option<PnpmConfig>,

    #[serde(skip_serializing_if = "is_default_true")]
    pub sync_project_workspace_dependencies: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_version_manager_config: Option<NodeVersionManager>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_node_version")]
    pub version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate]
    pub yarn: Option<YarnConfig>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        NodeConfig {
            add_engines_constraint: true,
            alias_package_names: None,
            bin_exec_args: vec![],
            dedupe_on_lockfile_change: true,
            dependency_version_format: NodeVersionFormat::Workspace,
            infer_tasks_from_scripts: false,
            npm: NpmConfig::default(),
            package_manager: NodePackageManager::default(),
            pnpm: None,
            sync_project_workspace_dependencies: true,
            sync_version_manager_config: None,
            version: None,
            yarn: None,
        }
    }
}
