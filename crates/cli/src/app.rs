// https://github.com/clap-rs/clap/tree/master/examples/derive_ref#app-attributes

use crate::commands::bin::BinTools;
use clap::{AppSettings, Parser, Subcommand};

#[derive(Debug, Subcommand)]
pub enum Commands {
    // mono bin <tool>
    #[clap(
        name = "bin",
        about = "Return an absolute path to a tool's binary within the toolchain.",
        long_about = "Return an absolute path to a tool's binary within the toolchain. If a tool has not been configured or installed, this will return a non-zero exit code with no value."
    )]
    Bin {
        #[clap(arg_enum, help = "The tool to query")]
        tool: BinTools,
    },

    // mono project <id>
    #[clap(
        name = "project",
        about = "Display information about a single project."
    )]
    Project {
        #[clap()]
        id: String,

        #[clap(long, help = "Print in JSON format")]
        json: bool,
    },

    // mono project-graph
    #[clap(
        name = "project-graph",
        about = "Display a graph of all projects, in multiple formats."
    )]
    ProjectGraph,

    // mono setup
    #[clap(
        name = "setup",
        about = "Setup the environment by installing all tools."
    )]
    Setup,

    // mono teardown
    #[clap(
        name = "teardown",
        about = "Teardown the environment by uninstalling all tools and deleting temp files."
    )]
    Teardown,
}

#[derive(Debug, Parser)]
#[clap(
    bin_name = "mono",
    name = "Monolith",
    about = "Take your monorepo to the moon!",
    version
)]
#[clap(global_setting(AppSettings::DisableHelpSubcommand))]
#[clap(global_setting(AppSettings::PropagateVersion))]
pub struct App {
    #[clap(subcommand)]
    pub command: Commands,
}
