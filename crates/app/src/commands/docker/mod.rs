mod docker_error;
mod file;
mod prune;
mod scaffold;
mod setup;

pub use file::*;
pub use prune::*;
pub use scaffold::*;
pub use setup::*;

use clap::Subcommand;
use moon_common::Id;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Subcommand)]
pub enum DockerCommands {
    #[command(name = "file", about = "Generate a default Dockerfile for a project.")]
    File(DockerFileArgs),

    #[command(
        name = "prune",
        about = "Remove extraneous files and folders within a Dockerfile."
    )]
    Prune,

    #[command(
        name = "scaffold",
        about = "Scaffold a repository skeleton for use within Dockerfile(s)."
    )]
    Scaffold(DockerScaffoldArgs),

    #[command(
        name = "setup",
        about = "Setup a Dockerfile by installing dependencies for necessary projects."
    )]
    Setup,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerManifest {
    pub focused_projects: FxHashSet<Id>,
    pub unfocused_projects: FxHashSet<Id>,
}

pub const MANIFEST_NAME: &str = "dockerManifest.json";
