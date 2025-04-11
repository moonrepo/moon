use crate::context::*;
use crate::prompts::*;
use moon_config::{DockerPruneConfig, DockerScaffoldConfig, UnresolvedVersionSpec, VersionSpec};
use moon_project::ProjectFragment;
use moon_task::TaskFragment;
use proto_pdk_api::ExecCommandInput;
use rustc_hash::FxHashMap;
use schematic::Schema;
use serde_json::Value;
use warpgate_api::{VirtualPath, api_struct, api_unit_enum};

// METADATA

api_struct!(
    /// Input passed to the `register_toolchain` function.
    pub struct RegisterToolchainInput {
        /// ID of the toolchain, as it was configured.
        pub id: String,
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

        /// The name of the lock file used for dependency installs.
        /// Will be used for project usage detection.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub lock_file_name: Option<String>,

        /// The name of the manifest file that contains project and
        /// dependency information. Will be used for project usage detection.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub manifest_file_name: Option<String>,

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

        /// Merged toolchain configuration.
        pub toolchain_config: serde_json::Value,
    }
);

api_struct!(
    /// Output returned from the `sync_workspace` and `sync_project` functions.
    #[serde(default)]
    pub struct SyncOutput {
        /// List of files that have been changed because of the sync.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub changed_files: Vec<VirtualPath>,

        /// Operations that were performed. This can be used to track
        /// metadata like time taken, result status, and more.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub operations_performed: Vec<Operation>,

        /// Whether the action was skipped or not.
        pub skipped: bool,
    }
);

// SETUP / TEARDOWN

api_struct!(
    /// Input passed to the `setup_toolchain` function.
    pub struct SetupToolchainInput {
        /// The unresolved version specification that this toolchain was configured with.
        pub configured_version: UnresolvedVersionSpec,

        /// Current moon context.
        pub context: MoonContext,

        /// Merged toolchain configuration.
        pub toolchain_config: serde_json::Value,

        /// The resolved version specification that was setup.
        pub version: Option<VersionSpec>,
    }
);

api_struct!(
    /// Output returned from the `setup_toolchain` function.
    pub struct SetupToolchainOutput {
        /// List of files that have been changed because of this action.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub changed_files: Vec<VirtualPath>,

        /// Whether the tool was installed or not. This field is ignored
        /// if set, and is defined on moon's side.
        pub installed: bool,
    }
);

api_struct!(
    /// Input passed to the `teardown_toolchain` function.
    pub struct TeardownToolchainInput {
        /// The unresolved version specification that this toolchain was configured with.
        pub configured_version: Option<UnresolvedVersionSpec>,

        /// Current moon context.
        pub context: MoonContext,

        /// Merged toolchain configuration.
        pub toolchain_config: serde_json::Value,
    }
);

api_struct!(
    /// Input passed to the `setup_environment` function.
    pub struct SetupEnvironmentInput {
        /// Current moon context.
        pub context: MoonContext,
        // TODO
    }
);

api_struct!(
    /// Output returned from the `setup_environment` function.
    pub struct SetupEnvironmentOutput {}
);

// DEPENDENCIES

api_struct!(
    /// Input passed to the `locate_dependencies_root` function.
    pub struct LocateDependenciesRootInput {
        /// Current moon context.
        pub context: MoonContext,

        /// The starting directory in which to locate the root.
        /// This is typically a project root.
        pub starting_dir: VirtualPath,
    }
);

api_struct!(
    /// Output returned from the `locate_dependencies_root` function.
    pub struct LocateDependenciesRootOutput {
        /// A list of relative globs for all members (packages, libs, etc)
        /// within the current dependencies workspace. If not defined,
        /// the current project is the root, or there is no workspace.
        pub members: Option<Vec<String>>,

        /// Virtual path to the located root. If no root was found,
        /// return `None` to abort any relevant operations.
        pub root: Option<VirtualPath>,
    }
);

api_struct!(
    /// Input passed to the `install_dependencies` function.
    pub struct InstallDependenciesInput {
        /// Current moon context.
        pub context: MoonContext,

        // Virtual path to the dependencies root. This is where
        // the lockfile and root manifest should exist.
        pub root: VirtualPath,
    }
);

api_struct!(
    /// Output returned from the `install_dependencies` function.
    pub struct InstallDependenciesOutput {
        /// The command to run in the dependencies root to dedupe
        /// dependencies. If not defined, will not dedupe.
        pub dedupe_command: Option<ExecCommandInput>,

        /// The command to run in the dependencies root to install
        /// dependencies. If not defined, will not install.
        pub install_command: Option<ExecCommandInput>,
    }
);

// RUN TASK

api_struct!(
    /// Input passed to the `hash_task_contents` function.
    pub struct HashTaskContentsInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Fragment of the project that the task belongs to.
        pub project: ProjectFragment,

        /// Fragment of the task being hashed.
        pub task: TaskFragment,

        /// Merged toolchain configuration.
        pub toolchain_config: serde_json::Value,
    }
);

api_struct!(
    /// Output returned from the `hash_task_contents` function.
    pub struct HashTaskContentsOutput {
        /// Contents that should be included during hash generation.
        pub contents: Vec<serde_json::Value>,
    }
);

// DOCKER

api_struct!(
    /// Input passed to the `define_docker_metadata` function.
    pub struct DefineDockerMetadataInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Merged toolchain configuration.
        pub toolchain_config: serde_json::Value,
    }
);

api_struct!(
    /// Output returned from the `define_docker_metadata` function.
    pub struct DefineDockerMetadataOutput {
        /// Default image to use when generating a `Dockerfile`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub default_image: Option<String>,

        /// List of files as globs to copy over during
        /// the scaffolding process. Applies to both project
        /// and workspace level scaffolding.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub project: Option<ProjectFragment>,
    }
);

api_struct!(
    /// Output returned from the `scaffold_docker` function.
    pub struct ScaffoldDockerOutput {
        /// List of files that were copied into the scaffold.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub copied_files: Vec<VirtualPath>,
    }
);

api_struct!(
    /// Input passed to the `prune_docker` function.
    pub struct PruneDockerInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Docker prune configuration.
        pub docker_config: DockerPruneConfig,
    }
);
