use crate::common::MoonContext;
use schematic::Schema;
use warpgate_api::*;

// METADATA

api_struct!(
    /// Input passed to the `register_extension` function.
    pub struct RegisterExtensionInput {
        /// ID of the toolchain, as it was configured.
        pub id: String,
    }
);

api_struct!(
    /// Output returned from the `register_extension` function.
    pub struct RegisterExtensionOutput {
        /// Schema shape of the tool's configuration.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub config_schema: Option<Schema>,

        /// Name of the extension.
        pub name: String,

        /// Optional description about what the extension does.
        pub description: Option<String>,

        /// Version of the plugin.
        pub plugin_version: String,
    }
);

// EXECUTE

api_struct!(
    /// Input passed to the `execute_extension` function.
    pub struct ExecuteExtensionInput {
        /// Custom arguments passed on the command line.
        pub args: Vec<String>,

        /// Current moon context.
        pub context: MoonContext,
    }
);
