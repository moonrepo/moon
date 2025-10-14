use crate::context::*;
use crate::host::*;
use crate::is_false;
use moon_config::{UnresolvedVersionSpec, Version, VersionSpec};
use moon_project::ProjectFragment;
use moon_task::TaskFragment;
use std::collections::BTreeMap;
use std::path::PathBuf;
use warpgate_api::{VirtualPath, api_enum, api_struct};

api_struct!(
    /// Input passed to the `define_requirements` function.
    pub struct DefineRequirementsInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Workspace toolchain configuration.
        pub toolchain_config: serde_json::Value,
    }
);

api_struct!(
    /// Output returned from the `define_requirements` function.
    #[serde(default)]
    pub struct DefineRequirementsOutput {
        /// Other toolchains that this toolchain requires and must be setup before hand.
        /// If targeting an unstable toolchain, the identifier must be prefixed with "unstable_".
        /// When the toolchain is stable, both identifiers will continue to work.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub requires: Vec<String>,
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

api_struct!(
    /// Input passed to the `locate_dependencies_root` function.
    pub struct LocateDependenciesRootInput {
        /// Current moon context.
        pub context: MoonContext,

        /// The starting directory in which to locate the root.
        /// This is typically a project root.
        pub starting_dir: VirtualPath,

        /// Workspace and project merged toolchain configuration,
        /// with the latter taking precedence.
        pub toolchain_config: serde_json::Value,
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
        pub packages: Vec<String>,

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

api_struct!(
    /// Input passed to the `parse_manifest` function.
    pub struct ParseManifestInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Virtual path to the manifest file.
        pub path: VirtualPath,

        /// Virtual path to the dependencies root. This is where
        /// the lockfile and root manifest should exist.
        pub root: VirtualPath,
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

api_struct!(
    /// Represents a dependency in a manifest file.
    #[serde(default)]
    pub struct ManifestDependencyConfig {
        /// The version is inherited from the workspace.
        #[serde(skip_serializing_if = "is_false")]
        pub inherited: bool,

        /// List of features enabled for this dependency.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub features: Vec<String>,

        /// Relative path to the dependency on the local file system.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub path: Option<PathBuf>,

        /// Unique reference, identifier, or specifier.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub reference: Option<String>,

        /// URL of the remote dependency.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub url: Option<String>,

        /// The defined version or requirement.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub version: Option<UnresolvedVersionSpec>,
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
        Config(ManifestDependencyConfig),
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

    /// Defines an explicit local path.
    pub fn path(path: PathBuf) -> Self {
        Self::Config(ManifestDependencyConfig {
            path: Some(path),
            ..Default::default()
        })
    }

    /// Defines an explicit remote URL.
    pub fn url(url: String) -> Self {
        Self::Config(ManifestDependencyConfig {
            url: Some(url),
            ..Default::default()
        })
    }

    /// Return an applicable version.
    pub fn get_version(&self) -> Option<&UnresolvedVersionSpec> {
        match self {
            ManifestDependency::Version(version) => Some(version),
            ManifestDependency::Config(cfg) => cfg.version.as_ref(),
            _ => None,
        }
    }

    /// Is the dependency version inherited.
    pub fn is_inherited(&self) -> bool {
        match self {
            ManifestDependency::Inherited(state) => *state,
            ManifestDependency::Config(cfg) => cfg.inherited,
            _ => false,
        }
    }
}

api_struct!(
    /// Input passed to the `parse_lock` function.
    pub struct ParseLockInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Virtual path to the lockfile.
        pub path: VirtualPath,

        /// Virtual path to the dependencies root. This is where
        /// the lockfile and root manifest should exist.
        pub root: VirtualPath,
    }
);

api_struct!(
    /// Output returned from the `parse_lock` function.
    #[serde(default)]
    pub struct ParseLockOutput {
        /// Map of all dependencies and their locked versions.
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        pub dependencies: BTreeMap<String, Vec<LockDependency>>,
    }
);

api_struct!(
    /// Represents a dependency definition in a lockfile.
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
