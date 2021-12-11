use crate::helpers::safe_exit;
use clap::ArgEnum;
use monolith_toolchain::Tool;
use monolith_workspace::Workspace;

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

pub async fn bin(workspace: &Workspace, tool_type: &BinTools) -> Result<(), clap::Error> {
    let toolchain = &workspace.toolchain;

    let tool: &dyn Tool = match tool_type {
        BinTools::Node => toolchain.get_node(),
        BinTools::Npm => toolchain.get_npm(),
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

    if tool.is_installed().await.is_err() {
        safe_exit(BinExitCodes::NotInstalled as i32);
    }

    println!("{}", tool.get_bin_path().display());

    Ok(())
}
