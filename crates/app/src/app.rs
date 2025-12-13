use crate::app_options::*;
use crate::commands::action_graph::ActionGraphArgs;
use crate::commands::bin::BinArgs;
use crate::commands::check::CheckArgs;
use crate::commands::ci::CiArgs;
use crate::commands::clean::CleanArgs;
use crate::commands::completions::CompletionsArgs;
use crate::commands::debug::DebugCommands;
use crate::commands::docker::DockerCommands;
use crate::commands::exec::ExecArgs;
use crate::commands::ext::ExtArgs;
use crate::commands::extension::ExtensionCommands;
use crate::commands::generate::GenerateArgs;
use crate::commands::hash::HashArgs;
use crate::commands::init::InitArgs;
use crate::commands::mcp::McpArgs;
use crate::commands::migrate::MigrateCommands;
use crate::commands::project::ProjectArgs;
use crate::commands::project_graph::ProjectGraphArgs;
use crate::commands::projects::ProjectsArgs;
use crate::commands::query::QueryCommands;
use crate::commands::run::RunArgs;
use crate::commands::sync::SyncCommands;
use crate::commands::task::TaskArgs;
use crate::commands::task_graph::TaskGraphArgs;
use crate::commands::tasks::TasksArgs;
use crate::commands::template::TemplateArgs;
use crate::commands::templates::TemplatesArgs;
use crate::commands::toolchain::ToolchainCommands;
use crate::systems::bootstrap;
use clap::builder::styling::{Color, Style, Styles};
use clap::{Parser, Subcommand};
use moon_cache::CacheMode;
use moon_env_var::GlobalEnvBag;
use starbase_styles::color::Color as ColorType;
use std::env;
use std::path::PathBuf;

#[cfg(windows)]
pub const EXE_NAME: &str = "moon.exe";

#[cfg(not(windows))]
pub const EXE_NAME: &str = "moon";

#[derive(Clone, Debug, Subcommand)]
pub enum Commands {
    #[command(
        alias = "ag",
        name = "action-graph",
        about = "Display an interactive dependency graph of actions."
    )]
    ActionGraph(ActionGraphArgs),

    #[command(
        name = "bin",
        about = "Return an absolute path to a binary within the toolchain."
    )]
    Bin(BinArgs),

    #[command(
        alias = "c",
        name = "check",
        about = "Run build and test related tasks for one or many projects."
    )]
    Check(CheckArgs),

    #[command(name = "ci", about = "Run all affected tasks in a CI environment.")]
    Ci(CiArgs),

    #[command(
        name = "clean",
        about = "Clean the workspace and delete any stale or invalid artifacts."
    )]
    Clean(CleanArgs),

    #[command(
        name = "completions",
        about = "Generate command completions for your current shell."
    )]
    Completions(CompletionsArgs),

    #[command(name = "debug", about = "Debug internals.", hide = true)]
    Debug {
        #[command(subcommand)]
        command: DebugCommands,
    },

    #[command(
        name = "docker",
        about = "Operations for integrating with Docker and Dockerfiles."
    )]
    Docker {
        #[command(subcommand)]
        command: DockerCommands,
    },

    #[command(
        name = "exec",
        about = "Low-level command for executing tasks in the action pipeline."
    )]
    Exec(ExecArgs),

    #[command(name = "ext", about = "Execute an extension plugin.")]
    Ext(ExtArgs),

    #[command(name = "extension", about = "Manage extension plugins.")]
    Extension {
        #[command(subcommand)]
        command: ExtensionCommands,
    },

    #[command(
        alias = "g",
        name = "generate",
        about = "Generate and scaffold files from a pre-defined template."
    )]
    Generate(GenerateArgs),

    #[command(
        alias = "h",
        name = "hash",
        about = "Inspect or diff the contents of a generated hashes."
    )]
    Hash(HashArgs),

    #[command(name = "init", about = "Initialize a new moon repository.")]
    Init(InitArgs),

    #[command(
        name = "mcp",
        about = "Start an MCP (model context protocol) server that can respond to AI agent requests."
    )]
    Mcp(McpArgs),

    #[command(
        name = "migrate",
        about = "Operations for migrating existing projects to moon."
    )]
    Migrate {
        #[command(subcommand)]
        command: MigrateCommands,
    },

    #[command(
        alias = "p",
        name = "project",
        about = "Display information about a single project."
    )]
    Project(ProjectArgs),

    #[command(
        alias = "pg",
        name = "project-graph",
        about = "Display an interactive graph of projects."
    )]
    ProjectGraph(ProjectGraphArgs),

    #[command(name = "projects", about = "Display a table of all projects.")]
    Projects(ProjectsArgs),

    #[command(
        alias = "q",
        name = "query",
        about = "Query information about moon, the environment, and more as JSON."
    )]
    Query {
        #[command(subcommand)]
        command: QueryCommands,
    },

    #[command(
        alias = "r",
        name = "run",
        about = "Run one or many tasks in the action pipeline."
    )]
    Run(RunArgs),

    #[command(
        name = "setup",
        about = "Setup the environment by installing all toolchains."
    )]
    Setup,

    #[command(
        name = "sync",
        about = "Sync the workspace and all projects to a healthy state."
    )]
    Sync {
        #[command(subcommand)]
        command: Option<SyncCommands>,
    },

    #[command(
        alias = "t",
        name = "task",
        about = "Display information about a single task."
    )]
    Task(TaskArgs),

    #[command(
        alias = "tg",
        name = "task-graph",
        about = "Display an interactive graph of tasks."
    )]
    TaskGraph(TaskGraphArgs),

    #[command(name = "tasks", about = "Display a table of all tasks.")]
    Tasks(TasksArgs),

    #[command(
        name = "teardown",
        about = "Teardown the environment by uninstalling all toolchains and deleting cache files."
    )]
    Teardown,

    #[command(
        name = "template",
        about = "Display information about a single template."
    )]
    Template(TemplateArgs),

    #[command(
        name = "templates",
        about = "List all templates that are available for code generation."
    )]
    Templates(TemplatesArgs),

    #[command(name = "toolchain", about = "Manage toolchain plugins.")]
    Toolchain {
        #[command(subcommand)]
        command: ToolchainCommands,
    },

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
    bin_name = EXE_NAME,
    name = "moon",
    about = "Take your repo to the moon!",
    version = env::var("MOON_VERSION").unwrap_or_default(),
    disable_help_subcommand = true,
    next_line_help = false,
    propagate_version = true,
    styles = create_styles()
)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        env = "MOON_CACHE",
        help = "Mode for cache operations",
        help_heading = "Global options",
        default_value_t
    )]
    pub cache: CacheMode,

    #[arg(
        long,
        global = true,
        env = "MOON_COLOR",
        help = "Force colored output",
        help_heading = "Global options"
    )]
    pub color: bool,

    #[arg(
        long,
        short = 'c',
        global = true,
        env = "MOON_CONCURRENCY",
        help = "Maximum number of threads to utilize",
        help_heading = "Global options"
    )]
    pub concurrency: Option<usize>,

    #[arg(
        long,
        global = true,
        env = "MOON_DUMP",
        help = "Dump a trace profile to the working directory",
        help_heading = "Global options"
    )]
    pub dump: bool,

    #[arg(
        value_enum,
        long,
        global = true,
        env = "MOON_LOG",
        help = "Lowest log level to output",
        help_heading = "Global options",
        default_value_t
    )]
    pub log: LogLevel,

    #[arg(
        long,
        global = true,
        env = "MOON_LOG_FILE",
        help = "Path to a file to write logs to",
        help_heading = "Global options"
    )]
    pub log_file: Option<PathBuf>,

    #[arg(
        long,
        short = 'q',
        global = true,
        env = "MOON_QUIET",
        help = "Hide all moon console output",
        help_heading = "Global options"
    )]
    pub quiet: bool,

    #[arg(
        value_enum,
        long,
        global = true,
        env = "MOON_THEME",
        help = "Terminal theme to print with",
        help_heading = "Global options",
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
