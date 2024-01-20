// https://github.com/clap-rs/clap/tree/master/examples/derive_ref#app-attributes

use crate::commands::bin::BinArgs;
use crate::commands::check::CheckArgs;
use crate::commands::ci::CiArgs;
use crate::commands::clean::CleanArgs;
use crate::commands::completions::CompletionsArgs;
use crate::commands::docker::DockerScaffoldArgs;
use crate::commands::ext::ExtArgs;
use crate::commands::generate::GenerateArgs;
use crate::commands::graph::action::ActionGraphArgs;
use crate::commands::graph::project::ProjectGraphArgs;
use crate::commands::init::InitArgs;
use crate::commands::migrate::FromPackageJsonArgs;
use crate::commands::node::RunScriptArgs;
use crate::commands::project::ProjectArgs;
use crate::commands::query::{
    QueryHashArgs, QueryHashDiffArgs, QueryProjectsArgs, QueryTasksArgs, QueryTouchedFilesArgs,
};
use crate::commands::run::RunArgs;
use crate::commands::syncs::codeowners::SyncCodeownersArgs;
use crate::commands::syncs::hooks::SyncHooksArgs;
use crate::commands::task::TaskArgs;
use crate::enums::{CacheMode, LogLevel};
use clap::builder::styling::{Color, Style, Styles};
use clap::{Parser, Subcommand};
use moon_common::consts::BIN_NAME;
use starbase::State;
use starbase_styles::color::Color as ColorType;
use std::path::PathBuf;

#[derive(Clone, Debug, Subcommand)]
pub enum DockerCommands {
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

#[derive(Clone, Debug, Subcommand)]
pub enum MigrateCommands {
    #[command(
        name = "from-package-json",
        about = "Migrate `package.json` scripts and dependencies to `moon.yml`."
    )]
    FromPackageJson(FromPackageJsonArgs),

    #[command(
        name = "from-turborepo",
        about = "Migrate `turbo.json` to moon configuration files."
    )]
    FromTurborepo,
}

#[derive(Clone, Debug, Subcommand)]
pub enum NodeCommands {
    #[command(
        name = "run-script",
        about = "Run a `package.json` script within a project."
    )]
    RunScript(RunScriptArgs),
}

#[derive(Clone, Debug, Subcommand)]
pub enum QueryCommands {
    #[command(
        name = "hash",
        about = "Inspect the contents of a generated hash.",
        long_about = "Inspect the contents of a generated hash, and display all sources and inputs that were used to generate it."
    )]
    Hash(QueryHashArgs),

    #[command(
        name = "hash-diff",
        about = "Query the difference between two hashes.",
        long_about = "Query the difference between two hashes. The left differences will be printed in green, while the right in red, and equal lines in white."
    )]
    HashDiff(QueryHashDiffArgs),

    #[command(
        name = "projects",
        about = "Query for projects within the project graph.",
        long_about = "Query for projects within the project graph. All options support regex patterns."
    )]
    Projects(QueryProjectsArgs),

    #[command(name = "tasks", about = "List all available projects & their tasks.")]
    Tasks(QueryTasksArgs),

    #[command(
        name = "touched-files",
        about = "Query for touched files between revisions."
    )]
    TouchedFiles(QueryTouchedFilesArgs),
}

#[derive(Clone, Debug, Subcommand)]
pub enum SyncCommands {
    #[command(
        name = "codeowners",
        about = "Aggregate and sync code owners to a `CODEOWNERS` file."
    )]
    Codeowners(SyncCodeownersArgs),

    #[command(
        name = "hooks",
        about = "Generate and sync hook scripts for the workspace configured VCS."
    )]
    Hooks(SyncHooksArgs),

    #[command(
        name = "projects",
        about = "Sync all projects and configs in the workspace."
    )]
    Projects,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Commands {
    #[command(
        name = "completions",
        about = "Generate command completions for your current shell."
    )]
    Completions(CompletionsArgs),

    // ENVIRONMENT

    // moon init
    #[command(
        name = "init",
        about = "Initialize a new moon repository, or a new toolchain, by scaffolding config files."
    )]
    Init(InitArgs),

    // TOOLCHAIN

    // moon bin <tool>
    #[command(
        name = "bin",
        about = "Return an absolute path to a tool's binary within the toolchain.",
        long_about = "Return an absolute path to a tool's binary within the toolchain. If a tool has not been configured or installed, this will return a non-zero exit code with no value."
    )]
    Bin(BinArgs),

    // moon node <command>
    #[command(name = "node", about = "Special Node.js commands.")]
    Node {
        #[command(subcommand)]
        command: NodeCommands,
    },

    // moon setup
    #[command(
        name = "setup",
        about = "Setup the environment by installing all tools."
    )]
    Setup,

    // moon teardown
    #[command(
        name = "teardown",
        about = "Teardown the environment by uninstalling all tools and deleting temp files."
    )]
    Teardown,

    // PROJECTS

    // moon action-graph [target]
    #[command(
        alias = "ag",
        name = "action-graph",
        about = "Display an interactive dependency graph of all tasks and actions."
    )]
    ActionGraph(ActionGraphArgs),

    // moon dep-graph [target]
    #[command(
        name = "dep-graph",
        about = "Display an interactive dependency graph of all tasks and actions.",
        alias = "dg",
        hide = true
    )]
    DepGraph(ActionGraphArgs),

    // moon project <id>
    #[command(
        name = "project",
        about = "Display information about a single project.",
        alias = "p"
    )]
    Project(ProjectArgs),

    // moon project-graph [id]
    #[command(
        name = "project-graph",
        about = "Display an interactive graph of projects.",
        alias = "pg"
    )]
    ProjectGraph(ProjectGraphArgs),

    #[command(name = "sync", about = "Sync the workspace to a healthy state.")]
    Sync {
        #[command(subcommand)]
        command: Option<SyncCommands>,
    },

    // moon task <target>
    #[command(
        name = "task",
        about = "Display information about a single task.",
        alias = "t"
    )]
    Task(TaskArgs),

    // GENERATOR

    // moon generate
    #[command(
        name = "generate",
        about = "Generate and scaffold files from a pre-defined template.",
        alias = "g"
    )]
    Generate(GenerateArgs),

    // RUNNER

    // moon check
    #[command(
        name = "check",
        about = "Run all build and test related tasks for the current project.",
        alias = "c"
    )]
    Check(CheckArgs),

    // moon ci
    #[command(
        name = "ci",
        about = "Run all affected projects and tasks in a CI environment."
    )]
    Ci(CiArgs),

    // moon run [...targets]
    #[command(
        name = "run",
        about = "Run one or many project tasks and their dependent tasks.",
        alias = "r"
    )]
    Run(RunArgs),

    // PLUGINS

    // moon ext
    #[command(name = "ext", about = "Execute an extension plugin.")]
    Ext(ExtArgs),

    // OTHER

    // moon clean
    #[command(
        name = "clean",
        about = "Clean the workspace and delete any stale or invalid artifacts."
    )]
    Clean(CleanArgs),

    // moon docker <operation>
    #[command(
        name = "docker",
        about = "Operations for integrating with Docker and Dockerfile(s)."
    )]
    Docker {
        #[command(subcommand)]
        command: DockerCommands,
    },

    // moon migrate <operation>
    #[command(
        name = "migrate",
        about = "Operations for migrating existing projects to moon.",
        rename_all = "camelCase"
    )]
    Migrate {
        #[command(subcommand)]
        command: MigrateCommands,

        #[arg(
            long,
            global = true,
            help = "Disable the check for touched/dirty files"
        )]
        skip_touched_files_check: bool,
    },

    // moon query <operation>
    #[command(
        name = "query",
        about = "Query information about moon, the environment, and pipeline.",
        long_about = "Query information about moon, the environment, and pipeline. Each operation can output JSON so that it may be consumed easily."
    )]
    Query {
        #[command(subcommand)]
        command: QueryCommands,
    },

    // moon upgrade
    #[command(
        alias = "up",
        name = "upgrade",
        about = "Upgrade to the latest version of moon."
    )]
    Upgrade,
}

fn fg(ty: ColorType) -> Style {
    Style::new().fg_color(Some(Color::from(ty as u8)))
}

fn create_styles() -> Styles {
    Styles::default()
        .error(fg(ColorType::Red))
        .header(Style::new().bold())
        .invalid(fg(ColorType::Yellow))
        .literal(fg(ColorType::Purple)) // args, options, etc
        .placeholder(fg(ColorType::GrayLight))
        .usage(fg(ColorType::Pink).bold())
        .valid(fg(ColorType::Green))
}

#[derive(Clone, Debug, Parser, State)]
#[command(
    bin_name = BIN_NAME,
    name = "moon",
    about = "Take your repo to the moon!",
    version,
    disable_help_subcommand = true,
    next_line_help = false,
    propagate_version = true,
    rename_all = "camelCase",
    styles = create_styles()
)]
pub struct App {
    #[arg(
        value_enum,
        long,
        global = true,
        env = "MOON_CACHE",
        help = "Mode for cache operations",
        default_value_t
    )]
    pub cache: CacheMode,

    #[arg(long, global = true, help = "Force colored output for moon")]
    pub color: bool,

    #[arg(
        long,
        short = 'c',
        global = true,
        env = "MOON_CONCURRENCY",
        help = "Maximum number of threads to utilize"
    )]
    pub concurrency: Option<usize>,

    #[arg(
        value_enum,
        long,
        global = true,
        env = "MOON_LOG",
        help = "Lowest log level to output",
        default_value_t
    )]
    pub log: LogLevel,

    #[arg(
        long,
        global = true,
        env = "MOON_LOG_FILE",
        help = "Path to a file to dump the moon logs"
    )]
    pub log_file: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

impl App {
    pub fn global_args(&self) -> GlobalArgs {
        GlobalArgs {
            concurrency: self.concurrency,
        }
    }
}

#[derive(State)]
pub struct GlobalArgs {
    pub concurrency: Option<usize>,
}
