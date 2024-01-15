use serde::{Deserialize, Serialize};
use warpgate_api::VirtualPath;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
/// Information about the current state of the tool.
pub struct ExtensionContext {
    /// Virtual path to the current working directory.
    pub working_dir: VirtualPath,

    /// Virtual path to the moon workspace root.
    pub workspace_root: VirtualPath,
}
