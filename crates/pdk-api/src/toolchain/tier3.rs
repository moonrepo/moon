use crate::context::*;
use crate::is_false;
use moon_config::{UnresolvedVersionSpec, VersionSpec};
use std::path::PathBuf;
use warpgate_api::api_struct;

pub use proto_pdk_api::{
    DownloadPrebuiltInput, DownloadPrebuiltOutput, LoadVersionsInput, LoadVersionsOutput,
    LocateExecutablesInput, LocateExecutablesOutput, NativeInstallInput, NativeInstallOutput,
    NativeUninstallInput, NativeUninstallOutput, RegisterToolInput, RegisterToolOutput,
    ResolveVersionInput, ResolveVersionOutput, UnpackArchiveInput,
};

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
