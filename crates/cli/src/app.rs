// https://github.com/clap-rs/clap/tree/master/examples/derive_ref#app-attributes

use crate::commands::bin::BinTools;
use crate::commands::run::RunStatus;
use clap::{ArgEnum, Parser, Subcommand};
use moon_project::TargetID;
use moon_terminal::output::label_moon;

#[derive(ArgEnum, Clone, Debug)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    // moon bin <tool>
    #[clap(
        name = "bin",
        about = "Return an absolute path to a tool's binary within the toolchain.",
        long_about = "Return an absolute path to a tool's binary within the toolchain. If a tool has not been configured or installed, this will return a non-zero exit code with no value.",
        next_help_heading = "Environment"
    )]
    Bin {
        #[clap(arg_enum, help = "The tool to query")]
        tool: BinTools,
    },

    // moon setup
    #[clap(
        name = "setup",
        about = "Setup the environment by installing all tools."
    )]
    Setup,

    // moon teardown
    #[clap(
        name = "teardown",
        about = "Teardown the environment by uninstalling all tools and deleting temp files."
    )]
    Teardown,

    // moon project <id>
    #[clap(
        name = "project",
        about = "Display information about a single project.",
        next_help_heading = "Projects"
    )]
    Project {
        #[clap(help = "ID of project to display")]
        id: String,

        #[clap(long, help = "Print in JSON format")]
        json: bool,
    },

    // moon project-graph [id]
    #[clap(
        name = "project-graph",
        about = "Display a graph of projects in DOT format."
    )]
    ProjectGraph {
        #[clap(help = "ID of project to *only* graph")]
        id: Option<String>,
    },

    // moon run [...targets]
    #[clap(
        name = "run",
        about = "Run a project task and all its dependent tasks.",
        next_help_heading = "Tasks"
    )]
    Run {
        #[clap(help = "List of targets (project:task) to run")]
        targets: Vec<TargetID>,

        #[clap(long, help = "Only run target it affected by changed files")]
        affected: bool,

        #[clap(arg_enum, long, help = "Determine affected files based on this status")]
        status: Option<RunStatus>,
    },
}

#[derive(Debug, Parser)]
#[clap(
    bin_name = "moon",
    name = label_moon(),
    about = "Take your monorepo to the moon!",
    version
)]
#[clap(
    disable_colored_help = true,
    disable_help_subcommand = true,
    dont_collapse_args_in_usage = true,
    propagate_version = true
)]
pub struct App {
    #[clap(arg_enum, long, short = 'L', help = "Lowest log level to output")]
    pub log_level: Option<LogLevel>,

    #[clap(subcommand)]
    pub command: Commands,
}
