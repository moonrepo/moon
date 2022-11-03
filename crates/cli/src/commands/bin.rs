use crate::helpers::load_workspace;
use clap::ValueEnum;
use moon_terminal::safe_exit;
use moon_toolchain::{Executable, Installable};

#[derive(ValueEnum, Clone, Debug)]
#[value(rename_all = "lowercase")]
pub enum BinTool {
    Node,
    Npm,
    Pnpm,
    Yarn,
}

enum BinExitCodes {
    NotConfigured = 1,
    NotInstalled = 2,
}

async fn is_installed<T: Send + Sync>(tool: &dyn Installable<T>, parent: &T) {
    let installed = tool.is_installed(parent, true).await;

    if installed.is_err() || !installed.unwrap() {
        safe_exit(BinExitCodes::NotInstalled as i32);
    }
}

fn not_configured() -> ! {
    safe_exit(BinExitCodes::NotConfigured as i32);
}

fn log_bin_path<T: Send + Sync>(tool: &dyn Executable<T>) {
    println!("{}", tool.get_bin_path().display());
}

pub async fn bin(tool_type: &BinTool) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;
    let toolchain = &workspace.toolchain;

    match tool_type {
        BinTool::Node => {
            let node = toolchain.node.get()?;

            is_installed(node, &()).await;
            log_bin_path(node);
        }
        BinTool::Npm | BinTool::Pnpm | BinTool::Yarn => {
            let node = toolchain.node.get()?;

            match tool_type {
                BinTool::Pnpm => match node.get_pnpm() {
                    Some(pnpm) => {
                        is_installed(pnpm, node).await;
                        log_bin_path(pnpm);
                    }
                    None => not_configured(),
                },
                BinTool::Yarn => match node.get_yarn() {
                    Some(yarn) => {
                        is_installed(yarn, node).await;
                        log_bin_path(yarn);
                    }
                    None => not_configured(),
                },
                _ => {
                    let npm = node.get_npm();

                    is_installed(npm, node).await;
                    log_bin_path(npm);
                }
            };
        }
    };

    Ok(())
}
