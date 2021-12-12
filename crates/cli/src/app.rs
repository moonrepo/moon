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
        #[clap(arg_enum, help = "The tool to query.")]
        tool: BinTools,
    },

    // mono setup
    #[clap(
        name = "setup",
        about = "Setup the environment by installing all necessary tools."
    )]
    Setup,

    // mono teardown
    #[clap(
        name = "teardown",
        about = "Teardown the environment by uninstalling all tools and deleting temporary files."
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

//         SubCommand::with_name("run")
//             .about("Run a task within a project.")
//             .arg(
//                 Arg::with_name("target")
//                     .help("The task target to run.")
//                     .index(1)
//                     .required(true),
//             ),
