mod app;
mod commands;
mod helpers;

use app::{App, Commands};
use clap::Parser;
use commands::bin::bin;
use commands::project::project;
use commands::setup::setup;
use commands::teardown::teardown;
use monolith_workspace::Workspace;

#[tokio::main]
async fn main() {
    // Create app and parse arguments
    let args = App::parse();

    // Load the workspace
    let workspace = Workspace::load().unwrap(); // TODO error

    // println!("{:#?}", workspace);
    // println!("{:#?}", args);

    // Match and run subcommand
    match &args.command {
        Commands::Bin { tool } => {
            bin(&workspace, tool).await.unwrap(); // TODO error
        }
        Commands::Project { id, json } => {
            project(&workspace, id, json).await.unwrap(); // TODO error
        }
        Commands::ProjectGraph => {}
        Commands::Setup => {
            setup(&workspace).await.unwrap(); // TODO error
        }
        Commands::Teardown => {
            teardown(&workspace).await.unwrap(); // TODO error
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use std::env;

    pub fn create_test_command(fixture: &str) -> Command {
        let mut path = env::current_dir().unwrap();
        path.push("../../tests/fixtures");
        path.push(fixture);

        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        cmd.current_dir(path.canonicalize().unwrap());
        cmd
    }
}
