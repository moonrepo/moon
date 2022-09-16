// https://github.com/clap-rs/clap/tree/master/examples/derive_ref#app-attributes

use std::path::PathBuf;

use crate::commands::bin::BinTools;
use crate::commands::init::{InheritProjectsAs, PackageManager};
use crate::enums::{CacheMode, LogLevel, TouchedStatus};
use clap::{Parser, Subcommand};
use moon_action::ProfileType;
use moon_config::ProjectID;
use moon_task::TargetID;
use moon_terminal::label_moon;

pub const BIN_NAME: &str = if cfg!(windows) { "moon.exe" } else { "moon" };

const HEADING_AFFECTED: &str = "Affected by changes";
const HEADING_DEBUGGING: &str = "Debugging";
const HEADING_PARALLELISM: &str = "Parallelism and distribution";

#[derive(Debug, Subcommand)]
pub enum DockerCommands {
    #[clap(
        name = "scaffold",
        about = "Scaffold a repository skeleton for use within Dockerfile COPY commands."
    )]
    Scaffold {
        #[clap(required = true, help = "List of project IDs to copy sources for")]
        ids: Vec<ProjectID>,
    },
}

#[derive(Debug, Subcommand)]
pub enum MigrateCommands {
    #[clap(
        name = "from-package-json",
        about = "Migrate `package.json` scripts and dependencies to `moon.yml`."
    )]
    FromPackageJson {
        #[clap(help = "ID of project to migrate")]
        id: ProjectID,
    },
}

#[derive(Debug, Subcommand)]
pub enum NodeCommands {
    #[clap(
        name = "run-script",
        about = "Run a `package.json` script within a project."
    )]
    RunScript {
        #[clap(help = "Name of the script")]
        name: String,

        #[clap(long, help = "ID of project to run in")]
        project: Option<ProjectID>,
    },
}

#[derive(Debug, Subcommand)]
pub enum QueryCommands {
    #[clap(
        name = "projects",
        about = "Query for projects within the project graph.",
        long_about = "Query for projects within the project graph. All options support regex patterns."
    )]
    Projects {
        #[clap(long, help = "Filter projects that match this alias")]
        alias: Option<String>,

        #[clap(
            long,
            help = "Filter projects that are affected based on touched files"
        )]
        affected: bool,

        #[clap(long, help = "Filter projects that match this ID")]
        id: Option<String>,

        #[clap(long, help = "Filter projects of this programming language")]
        language: Option<String>,

        #[clap(long, help = "Filter projects that match this source path")]
        source: Option<String>,

        #[clap(long, help = "Filter projects that have the following tasks")]
        tasks: Option<String>,

        #[clap(long = "type", help = "Filter projects of this type")]
        type_of: Option<String>,
    },

    #[clap(
        name = "touched-files",
        about = "Query for touched files between revisions.",
        rename_all = "camelCase"
    )]
    TouchedFiles {
        #[clap(long, help = "Base branch, commit, or revision to compare against")]
        base: Option<String>,

        #[clap(
            long,
            help = "When on the default branch, compare against the previous revision"
        )]
        default_branch: bool,

        #[clap(long, help = "Current branch, commit, or revision to compare with")]
        head: Option<String>,

        #[clap(long, help = "Gather files from you local state instead of upstream")]
        local: bool,

        #[clap(
            value_enum,
            long,
            help = "Filter files based on a touched status",
            default_value_t
        )]
        status: TouchedStatus,
    },
}

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
            value_enum,
            long,
            help = "Inherit projects from `package.json` workspaces",
            default_value_t
        )]
        inherit_projects: InheritProjectsAs,

        #[clap(
            value_enum,
            long,
            help = "Package manager to configure and use",
            default_value_t
        )]
        package_manager: PackageManager,

        #[clap(long, help = "Skip prompts and use default values")]
        yes: bool,
    },

    // TOOLCHAIN

    // moon bin <tool>
    #[clap(
        name = "bin",
        about = "Return an absolute path to a tool's binary within the toolchain.",
        long_about = "Return an absolute path to a tool's binary within the toolchain. If a tool has not been configured or installed, this will return a non-zero exit code with no value."
    )]
    Bin {
        #[clap(value_enum, help = "The tool to query")]
        tool: BinTools,
    },

    // moon node <command>
    #[clap(name = "node", about = "Special Node.js commands.")]
    Node {
        #[clap(subcommand)]
        command: NodeCommands,
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

    // moon dep-graph [target]
    #[clap(
        name = "dep-graph",
        about = "Display a dependency graph of all tasks and actions in DOT format.",
        alias = "dg"
    )]
    DepGraph {
        #[clap(help = "Target to *only* graph")]
        target: Option<String>,
    },

    // moon project <id>
    #[clap(
        name = "project",
        about = "Display information about a single project.",
        alias = "p"
    )]
    Project {
        #[clap(help = "ID of project to display")]
        id: ProjectID,

        #[clap(long, help = "Print in JSON format")]
        json: bool,
    },

    // moon project-graph [id]
    #[clap(
        name = "project-graph",
        about = "Display a graph of projects in DOT format.",
        alias = "pg"
    )]
    ProjectGraph {
        #[clap(help = "ID of project to *only* graph")]
        id: Option<ProjectID>,
    },

    #[clap(
        name = "sync",
        about = "Sync all projects in the workspace to a healthy state."
    )]
    Sync,

    // GENERATOR

    // moon generate
    #[clap(
        name = "generate",
        about = "Generate and scaffold files from a pre-defined template.",
        alias = "g",
        rename_all = "camelCase"
    )]
    Generate {
        #[clap(help = "Name of template to generate")]
        name: String,

        #[clap(help = "Destination path, relative from the current working directory")]
        dest: Option<String>,

        #[clap(
            long,
            help = "Use the default value of all variables instead of prompting"
        )]
        defaults: bool,

        #[clap(long, help = "Run entire generator process without writing files")]
        dry_run: bool,

        #[clap(long, help = "Force overwrite any existing files at the destination")]
        force: bool,

        #[clap(long, help = "Create a new template")]
        template: bool,

        // Variable args (after --)
        #[clap(last = true, help = "Arguments to define as variable values")]
        vars: Vec<String>,
    },

    // RUNNER

    // moon check
    #[clap(
        name = "check",
        about = "Run all build and test related tasks for the current project.",
        alias = "c"
    )]
    Check {
        #[clap(help = "List of project IDs to explicitly check")]
        ids: Vec<ProjectID>,
    },

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
        about = "Run one or many project tasks and their dependent tasks.",
        alias = "r"
    )]
    Run {
        #[clap(required = true, help = "List of targets (project:task) to run")]
        targets: Vec<TargetID>,

        #[clap(
            long,
            help = "Run dependents of the same task, as well as dependencies"
        )]
        dependents: bool,

        // Debugging
        #[clap(
            value_enum,
            long,
            help = "Record and generate a profile for ran tasks",
            help_heading = HEADING_DEBUGGING,
        )]
        profile: Option<ProfileType>,

        #[clap(long, help = "Generate a run report for the current actions")]
        report: bool,

        // Affected
        #[clap(
            long,
            help = "Only run target if affected by touched files",
            help_heading = HEADING_AFFECTED
        )]
        affected: bool,

        #[clap(
            value_enum,
            long,
            help = "Filter affected files based on a touched status",
            help_heading = HEADING_AFFECTED,
            default_value_t
        )]
        status: TouchedStatus,

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

    // OTHER

    // moon clean
    #[clap(
        name = "clean",
        about = "Clean the workspace and delete any stale or invalid artifacts."
    )]
    Clean {
        #[clap(long, default_value = "7 days", help = "Lifetime of cached artifacts")]
        lifetime: String,
    },

    // moon docker <operation>
    #[clap(name = "docker", about = "Operations for integrating with Docker.")]
    Docker {
        #[clap(subcommand)]
        command: DockerCommands,
    },

    // moon migrate <operation>
    #[clap(
        name = "migrate",
        about = "Operations for migrating existing projects to moon."
    )]
    Migrate {
        #[clap(subcommand)]
        command: MigrateCommands,
    },

    // moon query <operation>
    #[clap(
        name = "query",
        about = "Query information about moon, the environment, and pipeline.",
        long_about = "Query information about moon, the environment, and pipeline. Each operation will output JSON so that it may be consumed easily."
    )]
    Query {
        #[clap(subcommand)]
        command: QueryCommands,
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
        value_enum,
        long,
        env = "MOON_CACHE",
        help = "Mode for cache operations",
        default_value_t
    )]
    pub cache: CacheMode,

    #[clap(long, env = "MOON_COLOR", help = "Force colored output for moon")]
    pub color: bool,

    #[clap(
        value_enum,
        long,
        env = "MOON_LOG",
        help = "Lowest log level to output",
        default_value_t
    )]
    pub log: LogLevel,

    #[clap(
        long,
        env = "MOON_LOG_FILE",
        help = "Path to a file to dump the moon logs"
    )]
    pub log_file: Option<PathBuf>,

    #[clap(subcommand)]
    pub command: Commands,
}
