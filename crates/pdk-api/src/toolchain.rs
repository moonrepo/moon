use crate::context::*;
use crate::host::*;
use crate::is_false;
use crate::prompts::*;
use moon_config::{
    DockerPruneConfig, DockerScaffoldConfig, UnresolvedVersionSpec, Version, VersionSpec,
};
use moon_project::ProjectFragment;
use moon_task::TaskFragment;
use rustc_hash::FxHashMap;
use schematic::Schema;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use warpgate_api::{VirtualPath, api_enum, api_struct, api_unit_enum};

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

        /// The identifier to use when instantiating a proto tool instance
        /// to support tier 3 functionality. This is only required if the
        /// moon toolchain ID differs from the proto tool ID.
        // #[serde(skip_serializing_if = "Option::is_none")]
        // pub proto_tool_id: Option<String>,

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

// SETUP / TEARDOWN

api_struct!(
    /// Input passed to the `setup_toolchain` function.
    pub struct SetupToolchainInput {
        /// The unresolved version specification that the toolchain was
        /// configured with via the `version` setting.
        pub configured_version: Option<UnresolvedVersionSpec>,

        /// Current moon context.
        pub context: MoonContext,

        /// Workspace toolchain configuration.
        pub toolchain_config: serde_json::Value,

        /// The resolved version specification.
        pub version: Option<VersionSpec>,
    }
);

api_struct!(
    /// Output returned from the `setup_toolchain` function.
    #[serde(default)]
    pub struct SetupToolchainOutput {
        /// List of files that have been changed because of this action.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub changed_files: Vec<PathBuf>,

        /// Operations that were performed. This can be used to track
        /// metadata like time taken, result status, and more.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub operations: Vec<Operation>,

        /// Whether the tool was installed or not. This field is ignored
        /// if set, and is defined on the host side.
        #[serde(skip_serializing_if = "is_false")]
        pub installed: bool,
    }
);

api_struct!(
    /// Input passed to the `teardown_toolchain` function.
    pub struct TeardownToolchainInput {
        /// The unresolved version specification that the toolchain was
        /// configured with via the `version` setting.
        pub configured_version: Option<UnresolvedVersionSpec>,

        /// Current moon context.
        pub context: MoonContext,

        /// Workspace toolchain configuration.
        pub toolchain_config: serde_json::Value,

        /// The resolved version specification.
        pub version: Option<VersionSpec>,
    }
);

api_struct!(
    /// Input passed to the `setup_environment` function.
    pub struct SetupEnvironmentInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Virtual path to a global executables directory
        /// for the current toolchain.
        pub globals_dir: Option<VirtualPath>,

        /// The project if the dependencies and environment root
        /// are the project root (non-workspace).
        pub project: Option<ProjectFragment>,

        /// Virtual path to the dependencies root. This is where
        /// the lockfile and root manifest should exist.
        pub root: VirtualPath,

        /// Workspace and project merged toolchain configuration,
        /// with the latter taking precedence.
        pub toolchain_config: serde_json::Value,
    }
);

api_struct!(
    /// Output returned from the `setup_environment` function.
    #[serde(default)]
    pub struct SetupEnvironmentOutput {
        /// List of files that have been changed because of this action.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub changed_files: Vec<PathBuf>,

        /// List of commands to execute during setup.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub commands: Vec<ExecCommand>,

        /// Operations that were performed. This can be used to track
        /// metadata like time taken, result status, and more.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub operations: Vec<Operation>,
    }
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
    #[serde(default)]
    pub struct LocateDependenciesRootOutput {
        /// A list of relative globs for all members (packages, libs, etc)
        /// within the current dependencies workspace. If not defined,
        /// the current project is the root, or there is no workspace.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub members: Option<Vec<String>>,

        /// Virtual path to the located root. If no root was found,
        /// return `None` to abort any relevant operations.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub root: Option<PathBuf>,
    }
);

api_struct!(
    /// Input passed to the `install_dependencies` function.
    /// Requires `locate_dependencies_root`.
    pub struct InstallDependenciesInput {
        /// Current moon context.
        pub context: MoonContext,

        /// List of packages to only install dependencies for.
        pub packages: Option<Vec<String>>,

        /// Only install production dependencies.
        pub production: bool,

        /// The project if the dependencies and environment root
        /// are the project root (non-workspace).
        pub project: Option<ProjectFragment>,

        /// Virtual path to the dependencies root. This is where
        /// the lockfile and root manifest should exist.
        pub root: VirtualPath,

        /// Workspace and project merged toolchain configuration,
        /// with the latter taking precedence.
        pub toolchain_config: serde_json::Value,
    }
);

api_struct!(
    /// Output returned from the `install_dependencies` function.
    #[serde(default)]
    pub struct InstallDependenciesOutput {
        /// The command to run in the dependencies root to dedupe
        /// dependencies. If not defined, will not dedupe.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub dedupe_command: Option<ExecCommand>,

        /// The command to run in the dependencies root to install
        /// dependencies. If not defined, will not install.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub install_command: Option<ExecCommand>,

        /// Operations that were performed. This can be used to track
        /// metadata like time taken, result status, and more.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub operations: Vec<Operation>,
    }
);

// RUN TASK

api_struct!(
    /// Input passed to the `parse_manifest` function.
    pub struct ParseManifestInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Virtual path to the manifest file.
        pub path: VirtualPath,
    }
);

api_struct!(
    /// Output returned from the `parse_manifest` function.
    #[serde(default)]
    pub struct ParseManifestOutput {
        /// Build dependencies.
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        pub build_dependencies: BTreeMap<String, ManifestDependency>,

        /// Development dependencies.
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        pub dev_dependencies: BTreeMap<String, ManifestDependency>,

        /// Production dependencies.
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        pub dependencies: BTreeMap<String, ManifestDependency>,

        /// Peer dependencies.
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        pub peer_dependencies: BTreeMap<String, ManifestDependency>,

        /// Can the package be published or not.
        #[serde(skip_serializing_if = "is_false")]
        pub publishable: bool,

        /// Current version of the package.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub version: Option<Version>,
    }
);

api_enum!(
    /// Represents a dependency definition in a manifest file.
    #[serde(untagged)]
    pub enum ManifestDependency {
        /// Inherited from workspace.
        Inherited(bool),

        /// Only a version.
        Version(UnresolvedVersionSpec),

        /// Full configuration.
        Config {
            /// The version is inherited from the workspace.
            #[serde(default, skip_serializing_if = "is_false")]
            inherited: bool,

            /// List of features enabled for this dependency.
            #[serde(default, skip_serializing_if = "Vec::is_empty")]
            features: Vec<String>,

            /// The defined version or requirement.
            #[serde(default, skip_serializing_if = "Option::is_none")]
            version: Option<UnresolvedVersionSpec>,
        },
    }
);

impl ManifestDependency {
    /// Defines an explicit version or requirement.
    pub fn new(version: UnresolvedVersionSpec) -> Self {
        Self::Version(version)
    }

    /// Inherits a version from the workspace.
    pub fn inherited() -> Self {
        Self::Inherited(true)
    }

    /// Return an applicable version.
    pub fn get_version(&self) -> Option<&UnresolvedVersionSpec> {
        match self {
            ManifestDependency::Version(version) => Some(version),
            ManifestDependency::Config { version, .. } => version.as_ref(),
            _ => None,
        }
    }

    /// Is the dependency version inherited.
    pub fn is_inherited(&self) -> bool {
        match self {
            ManifestDependency::Inherited(state) => *state,
            ManifestDependency::Config { inherited, .. } => *inherited,
            _ => false,
        }
    }
}

api_struct!(
    /// Input passed to the `parse_lock` function.
    pub struct ParseLockInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Virtual path to the lock file.
        pub path: VirtualPath,
    }
);

api_struct!(
    /// Output returned from the `parse_lock` function.
    #[serde(default)]
    pub struct ParseLockOutput {
        /// Map of all dependencies and their locked versions.
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        pub dependencies: BTreeMap<String, Vec<LockDependency>>,

        /// Map of all packages within the current workspace.
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        pub packages: BTreeMap<String, Option<Version>>,
    }
);

api_struct!(
    /// Represents a dependency definition in a lock file.
    #[serde(default)]
    pub struct LockDependency {
        /// A unique hash: checksum, integrity, etc.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub hash: Option<String>,

        /// General metadata.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub meta: Option<String>,

        /// The version requirement.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub req: Option<UnresolvedVersionSpec>,

        /// The resolved version.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub version: Option<VersionSpec>,
    }
);

api_struct!(
    /// Input passed to the `hash_task_contents` function.
    pub struct HashTaskContentsInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Fragment of the project that the task belongs to.
        pub project: ProjectFragment,

        /// Fragment of the task being hashed.
        pub task: TaskFragment,

        /// Workspace and project merged toolchain configuration,
        /// with the latter taking precedence.
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
