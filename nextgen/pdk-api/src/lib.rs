pub mod extension;

pub use warpgate_api::*;

api_struct!(
    /// Information about the current moon workspace.
    pub struct MoonContext {
        /// Virtual path to the current working directory.
        pub working_dir: VirtualPath,

        /// Virtual path to the workspace root.
        pub workspace_root: VirtualPath,
    }
);

#[macro_export]
macro_rules! config_struct {
    ($struct:item) => {
        #[derive(Debug, serde::Deserialize)]
        #[serde(default, deny_unknown_fields, rename_all = "camelCase")]
        $struct
    };
}
