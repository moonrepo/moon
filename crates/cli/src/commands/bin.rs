use clap::ArgEnum;
use moon_terminal::helpers::safe_exit;
use moon_toolchain::{Executable, Installable, Toolchain};
use moon_workspace::Workspace;
use std::path::PathBuf;

#[derive(ArgEnum, Clone, Debug)]
pub enum BinTools {
    Node,
    Npm,
    Pnpm,
    Yarn,
}

enum BinExitCodes {
    NotConfigured = 1,
    NotInstalled = 2,
}

fn not_configured() -> ! {
    safe_exit(BinExitCodes::NotConfigured as i32);
}

pub async fn bin(tool_type: &BinTools) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;
    let toolchain = &workspace.toolchain;

    // Helper functions
    // let is_tool_installed = |tool: &dyn Installable<Toolchain>| async {
    //     let installed = tool.is_installed(toolchain, true).await;

    //     if installed.is_err() || !installed.unwrap() {
    //         safe_exit(BinExitCodes::NotInstalled as i32);
    //     }

    //     tool
    // };

    // This is janky, but because of our trait generics its required
    // match tool_type {
    //     BinTools::Node => {}
    // }

    // // Check if tool is installed and configured first
    // let tool: &(dyn Installable<_>) = match tool_type {
    //     BinTools::Node => toolchain.get_node(),
    //     BinTools::Npm => toolchain.get_node().get_npm(),
    //     BinTools::Pnpm => match toolchain.get_node().get_pnpm() {
    //         Some(t) => t,
    //         None => not_configured(),
    //     },
    //     BinTools::Yarn => match toolchain.get_node().get_yarn() {
    //         Some(t) => t,
    //         None => not_configured(),
    //     },
    // };

    // let installed = tool.is_installed(toolchain, true).await;

    // if installed.is_err() || !installed.unwrap() {
    //     safe_exit(BinExitCodes::NotInstalled as i32);
    // }

    // // We must do this again since the methods come from different traits
    // let bin_path: &PathBuf = match tool_type {
    //     BinTools::Node => toolchain.get_node().get_bin_path(),
    //     BinTools::Npm => toolchain.get_node().get_npm().get_bin_path(),
    //     BinTools::Pnpm => toolchain.get_node().get_pnpm().unwrap().get_bin_path(),
    //     BinTools::Yarn => toolchain.get_node().get_yarn().unwrap().get_bin_path(),
    // };

    // println!("{}", bin_path.display());

    Ok(())
}
