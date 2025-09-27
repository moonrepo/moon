use crate::context::*;
use crate::is_false;
use crate::prompts::*;
use moon_common::Id;
use moon_config::{DockerPruneConfig, DockerScaffoldConfig};
use moon_project::ProjectFragment;
use rustc_hash::FxHashMap;
use schematic::Schema;
use serde_json::Value;
use std::path::PathBuf;
use warpgate_api::{VirtualPath, api_struct, api_unit_enum};

pub use proto_pdk_api::{
    DetectVersionInput, DetectVersionOutput, ParseVersionFileInput, ParseVersionFileOutput,
};

// METADATA

api_struct!(
    /// Input passed to the `register_toolchain` function.
    pub struct RegisterToolchainInput {
        /// ID of the toolchain, as it was configured.
        pub id: Id,
    }
);

api_struct!(
    /// Output returned from the `register_toolchain` function.
    #[serde(default)]
    pub struct RegisterToolchainOutput {
        /// A list of config file names/globs, excluding lockfiles and
        /// manifest, used by this toolchain. Will be used for project
        /// usage detection.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub config_file_globs: Vec<String>,

        /// Optional description about what the toolchain does.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,

        /// The name of executables provided by the toolchain.
        /// Will be used for task usage detection.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub exe_names: Vec<String>,

        /// The name(s) of the lock file used for dependency installs.
        /// Will be used for project usage detection.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub lock_file_names: Vec<String>,

        /// The name(s) of the manifest file that contains project and
        /// dependency information. Will be used for project usage detection.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub manifest_file_names: Vec<String>,

        /// Name of the toolchain.
        pub name: String,

        /// Version of the plugin.
        pub plugin_version: String,

        /// The name of the directory that contains installed dependencies.
        /// Will be used for detection.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub vendor_dir_name: Option<String>,
    }
);

pub type ConfigSchema = Schema;

api_struct!(
    /// Output returned from the `define_toolchain_config` function.
    pub struct DefineToolchainConfigOutput {
        /// Schema shape of the tool's configuration.
        pub schema: ConfigSchema,
    }
);

// INIT

api_struct!(
    /// Input passed to the `initialize_toolchain` function.
    pub struct InitializeToolchainInput {
        /// Current moon context.
        pub context: MoonContext,
    }
);

api_struct!(
    /// Output returned from the `initialize_toolchain` function.
    #[serde(default)]
    pub struct InitializeToolchainOutput {
        /// A URL to documentation about available configuration settings.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub config_url: Option<String>,

        /// Settings to include in the injected toolchain config file.
        /// Supports dot notation for the keys.
        #[serde(skip_serializing_if = "FxHashMap::is_empty")]
        pub default_settings: FxHashMap<String, Value>,

        /// A URL to documentation about the toolchain.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub docs_url: Option<String>,

        /// A list of questions to prompt the user about configuration
        /// settings and the values to inject.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub prompts: Vec<SettingPrompt>,
    }
);

// SYNC WORKSPACE / PROJECT

api_struct!(
    /// Input passed to the `sync_workspace` function.
    pub struct SyncWorkspaceInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Workspace toolchain configuration.
        pub toolchain_config: serde_json::Value,
    }
);

api_struct!(
    /// Input passed to the `sync_project` function.
    pub struct SyncProjectInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Other projects that the project being synced depends on.
        pub project_dependencies: Vec<ProjectFragment>,

        /// Fragment of the project being synced.
        pub project: ProjectFragment,

        /// Workspace and project merged toolchain configuration,
        /// with the latter taking precedence.
        pub toolchain_config: serde_json::Value,

        /// Workspace only toolchain configuration.
        pub toolchain_workspace_config: serde_json::Value,
    }
);

api_struct!(
    /// Output returned from the `sync_workspace` and `sync_project` functions.
    #[serde(default)]
    pub struct SyncOutput {
        /// List of files that have been changed because of the sync.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub changed_files: Vec<PathBuf>,

        /// Operations that were performed. This can be used to track
        /// metadata like time taken, result status, and more.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub operations: Vec<Operation>,

        /// Whether the action was skipped or not.
        #[serde(skip_serializing_if = "is_false")]
        pub skipped: bool,
    }
);

// DOCKER

api_struct!(
    /// Input passed to the `define_docker_metadata` function.
    pub struct DefineDockerMetadataInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Workspace and project merged toolchain configuration,
        /// with the latter taking precedence.
        pub toolchain_config: serde_json::Value,
    }
);

api_struct!(
    /// Output returned from the `define_docker_metadata` function.
    #[serde(default)]
    pub struct DefineDockerMetadataOutput {
        /// Default image to use when generating a `Dockerfile`.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub default_image: Option<String>,

        /// List of files as globs to copy over during
        /// the scaffolding process. Applies to both project
        /// and workspace level scaffolding.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub scaffold_globs: Vec<String>,
    }
);

api_unit_enum!(
    /// The different scaffolding phases.
    pub enum ScaffoldDockerPhase {
        /// Only config files (manifests, lockfiles, etc).
        #[default]
        Configs,
        /// All sources within a project.
        Sources,
    }
);

api_struct!(
    /// Input passed to the `scaffold_docker` function.
    pub struct ScaffoldDockerInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Docker scaffold configuration.
        pub docker_config: DockerScaffoldConfig,

        /// The directory in which to copy files from.
        pub input_dir: VirtualPath,

        /// The directory in which to copy files to.
        pub output_dir: VirtualPath,

        /// The current scaffolding phase.
        pub phase: ScaffoldDockerPhase,

        /// The project being scaffolding.
        pub project: ProjectFragment,

        /// Workspace and project merged toolchain configuration,
        /// with the latter taking precedence.
        pub toolchain_config: serde_json::Value,
    }
);

api_struct!(
    /// Output returned from the `scaffold_docker` function.
    #[serde(default)]
    pub struct ScaffoldDockerOutput {
        /// List of files that were copied into the scaffold.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub copied_files: Vec<PathBuf>,
    }
);

api_struct!(
    /// Input passed to the `prune_docker` function.
    /// Requires `locate_dependencies_root`.
    pub struct PruneDockerInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Docker prune configuration.
        pub docker_config: DockerPruneConfig,

        /// The focused projects within the current
        /// dependencies root.
        pub projects: Vec<ProjectFragment>,

        /// Virtual path to the dependencies root. This is where
        /// the lockfile and root manifest should exist.
        pub root: VirtualPath,

        /// Workspace toolchain configuration.
        pub toolchain_config: serde_json::Value,
    }
);

api_struct!(
    /// Output returned from the `prune_docker` function.
    #[serde(default)]
    pub struct PruneDockerOutput {
        /// List of files that were changed during prune.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub changed_files: Vec<PathBuf>,
    }
);
