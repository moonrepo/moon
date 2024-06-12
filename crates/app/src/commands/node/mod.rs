mod run_script;

pub use run_script::{run_script, RunScriptArgs};

use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum NodeCommands {
    #[command(
        name = "run-script",
        about = "Run a `package.json` script within a project."
    )]
    RunScript(RunScriptArgs),
}
