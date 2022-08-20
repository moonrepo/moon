use crate::helpers::load_workspace;
use clap::ValueEnum;
use moon_terminal::safe_exit;
use moon_toolchain::{Executable, Installable};

#[derive(ValueEnum, Clone, Debug)]
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

pub async fn bin(tool_type: &BinTools) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;
    let toolchain = &workspace.toolchain;

    match tool_type {
        BinTools::Node => {
            let node = toolchain.get_node();

            is_installed(node, toolchain).await;
            log_bin_path(node);
        }
        BinTools::Npm | BinTools::Pnpm | BinTools::Yarn => {
            let node = toolchain.get_node();

            match tool_type {
                BinTools::Pnpm => match node.get_pnpm() {
                    Some(pnpm) => {
                        is_installed(pnpm, node).await;
                        log_bin_path(pnpm);
                    }
                    None => not_configured(),
                },
                BinTools::Yarn => match node.get_yarn() {
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
