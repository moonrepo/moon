mod app;
mod commands;

use app::create_app;
use commands::bin::{bin, BinOptions};
use monolith_workspace::Workspace;

#[tokio::main]
async fn main() {
    // Create app and parse arguments
    let app = create_app();
    let matches = app.get_matches();

    // Load the workspace
    let workspace = Workspace::load().unwrap();

    println!("{:#?}", workspace);
    println!("{:#?}", matches);

    // Match on a subcommand and branch logic
    match matches.subcommand() {
        ("run", Some(_run_matches)) => {
            println!("LOADING NODE");

            workspace
                .toolchain
                .load_tool(workspace.toolchain.get_node())
                .await
                .expect("NODE FAIL");

            println!("LOADING NPM");

            workspace
                .toolchain
                .load_tool(workspace.toolchain.get_npm())
                .await
                .expect("NPM FAIL");

            // println!("LOADING PACKAGE MANAGER");

            // workspace
            //     .toolchain
            //     .load_tool(workspace.toolchain.get_package_manager())
            //     .await
            //     .expect("PM FAIL");
        }
        ("bin", Some(bin_matches)) => {
            bin(
                &workspace,
                BinOptions {},
                bin_matches.value_of("tool").unwrap(),
            )
            .await
            .expect("BIN FAIL");
        }
        ("", None) => println!("Please select a command."),
        _ => unreachable!(),
    }
}
