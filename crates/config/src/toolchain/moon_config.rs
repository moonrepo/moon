use crate::config_struct;
use schematic::{Config, validate};

config_struct!(
    /// Configures how and where moon updates will be received.
    #[derive(Config)]
    pub struct MoonConfig {
        /// A secure URL to lookup the latest available version.
        #[setting(validate = validate::url_secure, default = "https://launch.moonrepo.app/moon/check_version")]
        pub manifest_url: String,

        /// A secure URL for downloading the moon binary itself.
        #[setting(validate = validate::url_secure, default = "https://github.com/moonrepo/moon/releases/latest/download/{file}")]
        pub download_url: String,
    }
);
