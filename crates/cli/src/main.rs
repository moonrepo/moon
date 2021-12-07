extern crate clap;

use clap::{crate_version, App, AppSettings, Arg, SubCommand};
use monolith_workspace::Workspace;

#[tokio::main]
async fn main() {
    // Build the app
    let app = App::new("Monolith")
        .bin_name("mono")
        .version(crate_version!())
        .about("First-class monorepo management.")
        .help_short("h")
        .version_short("v")
        .setting(AppSettings::DisableHelpSubcommand)
        .setting(AppSettings::GlobalVersion)
        .subcommand(
            SubCommand::with_name("run")
                .about("Run a task within a project.")
                .arg(
                    Arg::with_name("target")
                        .help("The task target to run.")
                        .index(1)
                        .required(true),
                ),
        );

    // Parse argv and return matches
    let matches = app.get_matches();
    let workspace = Workspace::load().unwrap();

    println!("{:#?}", workspace);

    // Match on a subcommand and branch logic
    match matches.subcommand_name() {
        Some("run") => {
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
        }
        None => println!("Please select a command."),
        _ => unreachable!(),
    }
}
