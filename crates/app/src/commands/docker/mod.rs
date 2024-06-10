mod docker_error;
mod prune;
mod scaffold;
mod setup;

pub use prune::*;
pub use scaffold::*;
pub use setup::*;

use moon_common::Id;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerManifest {
    pub focused_projects: FxHashSet<Id>,
    pub unfocused_projects: FxHashSet<Id>,
}

pub const MANIFEST_NAME: &str = "dockerManifest.json";
