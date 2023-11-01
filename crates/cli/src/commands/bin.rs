use clap::{Args, ValueEnum};
use moon_config::PlatformType;
use moon_node_tool::NodeTool;
use moon_platform::PlatformManager;
use moon_terminal::safe_exit;
use moon_tool::Tool;
use starbase::system;

#[derive(ValueEnum, Clone, Debug)]
#[value(rename_all = "lowercase")]
pub enum BinTool {
    // JavaScript
    Bun,
    Node,
    Npm,
    Pnpm,
    Yarn,
    // Rust
    Rust,
}

#[derive(Args, Clone, Debug)]
pub struct BinArgs {
    #[arg(value_enum, help = "The tool to query")]
    tool: BinTool,
}

enum BinExitCodes {
    NotConfigured = 1,
    NotInstalled = 2,
}

fn is_installed(tool: &dyn Tool) {
    if let Some(shim_path) = tool.get_shim_path() {
        println!("{}", shim_path.display());
    } else {
        match tool.get_bin_path() {
            Ok(path) => {
                println!("{}", path.display());
            }
            Err(_) => {
                safe_exit(BinExitCodes::NotInstalled as i32);
            }
        }
    }
}

fn not_configured() -> ! {
    safe_exit(BinExitCodes::NotConfigured as i32);
}

#[system]
pub async fn bin(args: ArgsRef<BinArgs>) {
    match &args.tool {
        BinTool::Bun => {
            let bun = PlatformManager::read().get(PlatformType::Bun)?.get_tool()?;

            is_installed(*bun);
        }
        BinTool::Node => {
            let node = PlatformManager::read()
                .get(PlatformType::Node)?
                .get_tool()?;

            is_installed(*node);
        }
        BinTool::Npm | BinTool::Pnpm | BinTool::Yarn => {
            let node = PlatformManager::read()
                .get(PlatformType::Node)?
                .get_tool()?
                .as_any();
            let node = node.downcast_ref::<NodeTool>().unwrap();

            match &args.tool {
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
        BinTool::Rust => {
            let rust = PlatformManager::read()
                .get(PlatformType::Rust)?
                .get_tool()?;

            is_installed(*rust);
        }
    };
}
