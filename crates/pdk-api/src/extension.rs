use crate::common::MoonContext;
use warpgate_api::*;

api_struct!(
    /// Input passed to the `execute_extension` function.
    pub struct ExecuteExtensionInput {
        /// Custom arguments passed on the command line.
        pub args: Vec<String>,

        /// Current moon context.
        pub context: MoonContext,
    }
);
