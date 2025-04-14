use crate::app_options::*;
use crate::commands::bin::BinArgs;
use crate::commands::check::CheckArgs;
use crate::commands::ci::CiArgs;
use crate::commands::clean::CleanArgs;
use crate::commands::completions::CompletionsArgs;
use crate::commands::debug::DebugCommands;
use crate::commands::docker::DockerCommands;
use crate::commands::ext::ExtArgs;
use crate::commands::generate::GenerateArgs;
use crate::commands::graph::action::ActionGraphArgs;
use crate::commands::graph::project::ProjectGraphArgs;
use crate::commands::graph::task::TaskGraphArgs;
use crate::commands::init::InitArgs;
use crate::commands::migrate::MigrateCommands;
use crate::commands::node::NodeCommands;
use crate::commands::project::ProjectArgs;
use crate::commands::query::QueryCommands;
use crate::commands::run::RunArgs;
use crate::commands::sync::SyncCommands;
use crate::commands::task::TaskArgs;
use crate::commands::templates::TemplatesArgs;
use crate::systems::bootstrap;
use clap::builder::styling::{Color, Style, Styles};
use clap::{Parser, Subcommand};
use moon_cache::CacheMode;
use moon_common::consts::BIN_NAME;
use moon_env_var::GlobalEnvBag;
use starbase_styles::color::Color as ColorType;
use std::env;
use std::path::PathBuf;

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

    #[command(name = "debug", about = "Debug internals.", hide = true)]
    Debug {
        #[command(subcommand)]
        command: DebugCommands,
    },

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

    // moon task-graph [id]
    #[command(
        name = "task-graph",
        about = "Display an interactive graph of tasks.",
        alias = "tg"
    )]
    TaskGraph(TaskGraphArgs),

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

    // moon templates
    #[command(
        name = "templates",
        about = "List all templates that are available for code generation."
    )]
    Templates(TemplatesArgs),

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

#[derive(Clone, Debug, Parser)]
#[command(
    bin_name = BIN_NAME,
    name = "moon",
    about = "Take your repo to the moon!",
    version = env::var("MOON_VERSION").unwrap_or_default(),
    disable_help_subcommand = true,
    next_line_help = false,
    propagate_version = true,
    rename_all = "camelCase",
    styles = create_styles()
)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        env = "MOON_CACHE",
        help = "Mode for cache operations",
        default_value_t
    )]
    pub cache: CacheMode,

    #[arg(long, global = true, help = "Force colored output")]
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
        long,
        global = true,
        env = "MOON_DUMP",
        help = "Dump a trace profile to the working directory"
    )]
    pub dump: bool,

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
        help = "Path to a file to write logs to"
    )]
    pub log_file: Option<PathBuf>,

    #[arg(
        long,
        short = 'q',
        global = true,
        env = "MOON_QUIET",
        help = "Hide all non-important terminal output"
    )]
    pub quiet: bool,

    #[arg(
        value_enum,
        long,
        global = true,
        env = "MOON_THEME",
        help = "Terminal theme to print with",
        default_value_t
    )]
    pub theme: AppTheme,

    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub fn setup_env_vars(&self) {
        bootstrap::setup_colors(self.color);

        let bag = GlobalEnvBag::instance();
        bag.set("STARBASE_LOG", self.log.to_string());
        bag.set("STARBASE_THEME", self.theme.to_string());

        if !bag.has("MOON_CACHE") {
            bag.set("MOON_CACHE", self.cache.to_string());
        }

        if !bag.has("MOON_LOG") {
            bag.set("MOON_LOG", self.log.to_string());
        }

        if !bag.has("MOON_THEME") {
            bag.set("MOON_THEME", self.theme.to_string());
        }

        if matches!(self.cache, CacheMode::Off | CacheMode::Write) {
            bag.set("PROTO_CACHE", "off");
        }

        if bag.should_debug_wasm() {
            bag.set("PROTO_WASM_LOG", "trace");
            bag.set("PROTO_DEBUG_WASM", "true");
            bag.set("EXTISM_DEBUG", "1");
            bag.set("EXTISM_ENABLE_WASI_OUTPUT", "1");
            bag.set("EXTISM_MEMDUMP", "wasm-plugin.mem");
            bag.set("EXTISM_COREDUMP", "wasm-plugin.core");
        }
    }
}
