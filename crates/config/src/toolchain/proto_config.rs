use crate::config_struct;
use schematic::{Config, DefaultValueResult};
use version_spec::VersionSpec;

fn default_version(_: &()) -> DefaultValueResult<VersionSpec> {
    Ok(VersionSpec::parse("0.53.2").ok())
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
