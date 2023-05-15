use crate::validate::validate_semver;
use schematic::{config_enum, Config};

config_enum!(
    pub enum NodeProjectAliasFormat {
        #[default]
        NameAndScope, // @scope/name
        NameOnly, // name
    }
);

config_enum!(
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
);

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

config_enum!(
    pub enum NodePackageManager {
        #[default]
        Npm,
        Pnpm,
        Yarn,
    }
);

config_enum!(
    pub enum NodeVersionManager {
        Nodenv,
        #[default]
        Nvm,
    }
);

#[derive(Config)]
pub struct NpmConfig {
    #[setting(validate = validate_semver)]
    pub version: Option<String>,
}

#[derive(Config)]
pub struct PnpmConfig {
    #[setting(validate = validate_semver)]
    pub version: Option<String>,
}

#[derive(Config)]
pub struct YarnConfig {
    pub plugins: Vec<String>,

    #[setting(validate = validate_semver)]
    pub version: Option<String>,
}

#[derive(Config)]
pub struct NodeConfig {
    #[setting(default = true)]
    pub add_engines_constraint: bool,

    #[deprecated]
    pub alias_package_names: NodeProjectAliasFormat,

    pub bin_exec_args: Vec<String>,

    #[setting(default = true)]
    pub dedupe_on_lockfile_change: bool,

    pub dependency_version_format: NodeVersionFormat,

    pub infer_tasks_from_scripts: bool,

    #[setting(nested)]
    pub npm: NpmConfig,

    pub package_manager: NodePackageManager,

    #[setting(nested)]
    pub pnpm: Option<PnpmConfig>,

    #[setting(default = true)]
    pub sync_project_workspace_dependencies: bool,

    pub sync_version_manager_config: Option<NodeVersionManager>,

    #[setting(validate = validate_semver)]
    pub version: Option<String>,

    #[setting(nested)]
    pub yarn: Option<YarnConfig>,
}
