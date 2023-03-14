mod prune;
mod scaffold;
mod setup;

pub const MANIFEST_NAME: &str = "dockerManifest.json";

pub use prune::prune;
pub use scaffold::{scaffold, DockerManifest};
pub use setup::setup;
