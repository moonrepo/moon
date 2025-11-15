use crate::common::{InitializePluginInput, InitializePluginOutput};
use crate::context::*;
use moon_common::Id;
use moon_config::{DockerPruneConfig, DockerScaffoldConfig, LanguageType};
use moon_project::ProjectFragment;
use schematic::Schema;
use std::path::PathBuf;
use warpgate_api::{VirtualPath, api_struct, api_unit_enum};

pub use proto_pdk_api::{
    DetectVersionInput, DetectVersionOutput, ParseVersionFileInput, ParseVersionFileOutput,
};

pub type InitializeToolchainInput = InitializePluginInput;
pub type InitializeToolchainOutput = InitializePluginOutput;

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

        /// The programming language this toolchain applies to. This value
        /// will be used for 2 purposes:
        ///   - If the project language is unknown, it will use this value
        ///     if the project is toolchain aware based on file detection.
        ///   - If the project language is defined, it will infer the
        ///     the toolchain from this value.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub language: Option<LanguageType>,

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

api_struct!(
    /// Output returned from the `define_toolchain_config` function.
    pub struct DefineToolchainConfigOutput {
        /// Schema shape of the toolchain's configuration.
        pub schema: Schema,
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

        /// List of files as globs to copy over during the scaffolding
        /// process, within both "configs" and "sources" phases.
        /// Applies to both project and workspace level scaffolding.
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
        pub project: Option<ProjectFragment>,

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
