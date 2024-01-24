use warpgate_api::*;

pub use crate::MoonContext;

api_struct!(
    /// Input passed to the `execute_extension` function.
    pub struct ExecuteExtensionInput {
        /// Custom arguments passed on the command line.
        pub args: Vec<String>,

        /// Current moon context.
        pub context: MoonContext,
    }
);
