// https://github.com/clap-rs/clap/tree/master/examples/derive_ref#app-attributes

use crate::commands::bin::BinTools;
use crate::commands::init::{InheritProjectsAs, PackageManager};
use crate::commands::run::RunStatus;
use crate::enums::{CacheMode, LogLevel};
use clap::{Parser, Subcommand};
use moon_project::TargetID;
use moon_terminal::output::label_moon;

pub const BIN_NAME: &str = if cfg!(windows) { "moon.exe" } else { "moon" };

const HEADING_AFFECTED: &str = "Affected by changes";
const HEADING_PARALLELISM: &str = "Parallelism and distribution";

#[derive(Debug, Subcommand)]
pub enum Commands {
    // ENVIRONMENT

    // moon init
    #[clap(
        name = "init",
        about = "Initialize a new moon repository and scaffold config files.",
        rename_all = "camelCase"
    )]
    Init {
        #[clap(help = "Destination to initialize in", default_value = ".")]
        dest: String,

        #[clap(long, help = "Overwrite existing configurations")]
        force: bool,

        #[clap(
            arg_enum,
            long,
            help = "Inherit projects from `package.json` workspaces",
            default_value_t
        )]
        inherit_projects: InheritProjectsAs,

        #[clap(
            arg_enum,
            long,
            help = "Package manager to configure and use",
            default_value_t
        )]
        package_manager: PackageManager,

        #[clap(long, help = "Skip prompts and use default values")]
        yes: bool,
    },

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

    // PROJECTS

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
        about = "Display a graph of projects in DOT format.",
        alias = "graph"
    )]
    ProjectGraph {
        #[clap(help = "ID of project to *only* graph")]
        id: Option<String>,
    },

    // JOBS

    // moon ci
    #[clap(
        name = "ci",
        about = "Run all affected projects and tasks in a CI environment.",
        rename_all = "camelCase"
    )]
    Ci {
        #[clap(long, help = "Base branch, commit, or revision to compare against")]
        base: Option<String>,

        #[clap(long, help = "Current branch, commit, or revision to compare with")]
        head: Option<String>,

        #[clap(long, help = "Index of the current job", help_heading = HEADING_PARALLELISM)]
        job: Option<usize>,

        #[clap(long, help = "Total amount of jobs to run", help_heading = HEADING_PARALLELISM)]
        job_total: Option<usize>,
    },

    // moon run [...targets]
    #[clap(
        name = "run",
        about = "Run a project task and all its dependent tasks."
    )]
    Run {
        #[clap(help = "Target (project:task) to run")]
        target: TargetID,

        #[clap(
            long,
            help = "Run dependents of the same task, as well as dependencies"
        )]
        dependents: bool,

        // Affected
        #[clap(
            long,
            help = "Only run target if affected by changed files",
            help_heading = HEADING_AFFECTED
        )]
        affected: bool,

        #[clap(
            arg_enum,
            long,
            help = "Filter affected files based on a change status",
            help_heading = HEADING_AFFECTED,
            default_value_t
        )]
        status: RunStatus,

        #[clap(
            long,
            help = "Determine affected against upstream by comparing against a base revision",
            help_heading = HEADING_AFFECTED
        )]
        upstream: bool,

        // Passthrough args (after --)
        #[clap(
            last = true,
            help = "Arguments to pass through to the underlying command"
        )]
        passthrough: Vec<String>,
    },
}

#[derive(Debug, Parser)]
#[clap(
    bin_name = BIN_NAME,
    name = label_moon(),
    about = "Take your repo to the moon!",
    version
)]
#[clap(
    disable_colored_help = true,
    disable_help_subcommand = true,
    dont_collapse_args_in_usage = true,
    propagate_version = true,
    next_line_help = false,
    rename_all = "camelCase"
)]
pub struct App {
    #[clap(
        arg_enum,
        long,
        env = "MOON_CACHE",
        help = "Mode for cache operations",
        default_value_t
    )]
    pub cache: CacheMode,

    #[clap(long, env = "MOON_COLOR", help = "Force colored output for moon")]
    pub color: bool,

    #[clap(
        arg_enum,
        long,
        env = "MOON_LOG",
        help = "Lowest log level to output",
        default_value_t
    )]
    pub log: LogLevel,

    #[clap(subcommand)]
    pub command: Commands,
}
