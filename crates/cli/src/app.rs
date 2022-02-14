// https://github.com/clap-rs/clap/tree/master/examples/derive_ref#app-attributes

use crate::commands::bin::BinTools;
use crate::commands::run_affected::RunStatus;
use clap::ArgEnum;
use clap::{AppSettings, Parser, Subcommand};
use moon_project::TargetID;

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
        long_about = "Return an absolute path to a tool's binary within the toolchain. If a tool has not been configured or installed, this will return a non-zero exit code with no value."
    )]
    Bin {
        #[clap(arg_enum, help = "The tool to query")]
        tool: BinTools,
    },

    // moon project <id>
    #[clap(
        name = "project",
        about = "Display information about a single project."
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

    // moon run [target]
    #[clap(
        name = "run",
        about = "Run a project task and all its dependent tasks."
    )]
    Run {
        #[clap(help = "Target (project:task) to run")]
        target: TargetID,
    },

    // moon run-affected [target]
    #[clap(
        name = "run-affected",
        about = "Run a project task if it has been affected by changed files."
    )]
    RunAffected {
        #[clap(help = "Target (project:task) to run")]
        target: TargetID,

        #[clap(arg_enum, long, help = "Determine affected files based on this status")]
        status: Option<RunStatus>,
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
}

#[derive(Debug, Parser)]
#[clap(
    bin_name = "moon",
    name = "Moon",
    about = "Take your monorepo to the moon!",
    version
)]
#[clap(global_setting(AppSettings::DisableColoredHelp))]
#[clap(global_setting(AppSettings::DisableHelpSubcommand))]
#[clap(global_setting(AppSettings::DontCollapseArgsInUsage))]
#[clap(global_setting(AppSettings::PropagateVersion))]
pub struct App {
    #[clap(arg_enum, long, short = 'L', help = "Lowest log level to output")]
    pub log_level: Option<LogLevel>,

    #[clap(subcommand)]
    pub command: Commands,
}
