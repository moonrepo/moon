mod app;
mod commands;

use app::{App, Commands};
use clap::Parser;
use commands::bin::bin;
use monolith_workspace::Workspace;

#[tokio::main]
async fn main() {
    // Create app and parse arguments
    let args = App::parse();

    // Load the workspace
    let workspace = Workspace::load().unwrap();

    println!("{:#?}", workspace);
    println!("{:#?}", args);

    // Match and run subcommand
    match &args.command {
        Commands::Bin { tool } => {
            bin(&workspace, tool).await.expect("BIN FAIL");
        }
    }
}
