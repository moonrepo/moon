use crate::helpers::safe_exit;
use clap::ArgEnum;
use moon_toolchain::Tool;
use moon_workspace::Workspace;

#[derive(ArgEnum, Clone, Debug)]
pub enum BinTools {
    Node,
    Npm,
    Npx,
    Pnpm,
    Yarn,
}

enum BinExitCodes {
    NotConfigured = 1,
    NotInstalled = 2,
}

pub async fn bin(tool_type: &BinTools) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load()?;
    let toolchain = &workspace.toolchain;

    let tool: &dyn Tool = match tool_type {
        BinTools::Node => toolchain.get_node(),
        BinTools::Npm => toolchain.get_npm(),
        BinTools::Npx => toolchain.get_npx(),
        BinTools::Pnpm => match toolchain.get_pnpm() {
            Some(t) => t,
            None => {
                safe_exit(BinExitCodes::NotConfigured as i32);
            }
        },
        BinTools::Yarn => match toolchain.get_yarn() {
            Some(t) => t,
            None => {
                safe_exit(BinExitCodes::NotConfigured as i32);
            }
        },
    };

    let installed = tool.is_installed().await;

    if installed.is_err() || !installed.unwrap() {
        safe_exit(BinExitCodes::NotInstalled as i32);
    }

    println!("{}", tool.get_bin_path().display());

    Ok(())
}
