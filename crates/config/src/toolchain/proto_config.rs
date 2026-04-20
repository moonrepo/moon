use crate::config_struct;
use schematic::{Config, DefaultValueResult};
use version_spec::VersionSpec;

pub const PROTO_CLI_VERSION: &str = "0.56.4";

fn default_version(_: &()) -> DefaultValueResult<VersionSpec> {
    Ok(VersionSpec::parse(PROTO_CLI_VERSION).ok())
}

config_struct!(
    /// Configures how moon integrates with proto.
    #[derive(Config)]
    pub struct ProtoConfig {
        /// The version of proto to download and install,
        /// and to use for installing and running other toolchains.
        #[setting(default = default_version)]
        pub version: VersionSpec,
    }
);
