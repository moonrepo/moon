use warpgate_api::*;

api_struct!(
    /// Information about the current state of the tool.
    pub struct ExtensionContext {
        /// Virtual path to the current working directory.
        pub working_dir: VirtualPath,

        /// Virtual path to the moon workspace root.
        pub workspace_root: VirtualPath,
    }
);
