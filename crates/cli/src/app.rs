// https://github.com/clap-rs/clap/tree/master/examples/derive_ref#app-attributes

use crate::commands::bin::BinTools;
use clap::{AppSettings, Parser, Subcommand};

#[derive(Debug, Subcommand)]
pub enum Commands {
    // mono bin <tool>
    #[clap(
        name = "bin",
        about = "Return an absolute path to a toolchain binary.",
        long_about = "Return an absolute path to a toolchain binary. If a tool has not been configured or installed, this will return an empty value with a non-zero exit code."
    )]
    Bin {
        #[clap(arg_enum, help = "The tool to query.")]
        tool: BinTools,
    },
}

#[derive(Debug, Parser)]
#[clap(
    bin_name = "mono",
    name = "Monolith",
    about = "First-class monorepo management.",
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
