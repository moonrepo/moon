use crate::helpers::AnyError;
use clap::ValueEnum;
use moon::load_workspace;
use moon_config::PlatformType;
use moon_node_tool::NodeTool;
use moon_terminal::safe_exit;
use moon_tool::Tool;

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

fn is_installed(tool: &dyn Tool) {
    match tool.get_bin_path() {
        Ok(path) => {
            println!("{}", path.display());
        }
        Err(_) => {
            safe_exit(BinExitCodes::NotInstalled as i32);
        }
    }
}

fn not_configured() -> ! {
    safe_exit(BinExitCodes::NotConfigured as i32);
}

pub async fn bin(tool_type: &BinTool) -> Result<(), AnyError> {
    let workspace = load_workspace().await?;

    match tool_type {
        BinTool::Node => {
            let node = workspace.platforms.get(PlatformType::Node)?.get_tool()?;

            is_installed(*node);
        }
        BinTool::Npm | BinTool::Pnpm | BinTool::Yarn => {
            let node = workspace
                .platforms
                .get(PlatformType::Node)?
                .get_tool()?
                .as_any();
            let node = node.downcast_ref::<NodeTool>().unwrap();

            match tool_type {
                BinTool::Npm => match node.get_npm() {
                    Ok(npm) => is_installed(npm),
                    Err(_) => not_configured(),
                },
                BinTool::Pnpm => match node.get_pnpm() {
                    Ok(pnpm) => is_installed(pnpm),
                    Err(_) => not_configured(),
                },
                BinTool::Yarn => match node.get_yarn() {
                    Ok(yarn) => is_installed(yarn),
                    Err(_) => not_configured(),
                },
                _ => {}
            };
        }
    };

    Ok(())
}
