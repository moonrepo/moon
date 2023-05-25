use clap::ValueEnum;
use miette::IntoDiagnostic;
use moon::load_workspace_with_toolchain;
use moon_config::PlatformType;
use moon_node_tool::NodeTool;
use moon_terminal::safe_exit;
use moon_tool::Tool;
use starbase::AppResult;

#[derive(ValueEnum, Clone, Debug)]
#[value(rename_all = "lowercase")]
pub enum BinTool {
    // JavaScript
    Node,
    Npm,
    Pnpm,
    Yarn,
    // Rust
    Rust,
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

pub async fn bin(tool_type: BinTool) -> AppResult {
    let workspace = load_workspace_with_toolchain().await.into_diagnostic()?;

    match tool_type {
        BinTool::Node => {
            let node = workspace
                .platforms
                .get(PlatformType::Node)
                .into_diagnostic()?
                .get_tool()
                .into_diagnostic()?;

            is_installed(*node);
        }
        BinTool::Npm | BinTool::Pnpm | BinTool::Yarn => {
            let node = workspace
                .platforms
                .get(PlatformType::Node)
                .into_diagnostic()?
                .get_tool()
                .into_diagnostic()?
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
        BinTool::Rust => {
            let rust = workspace
                .platforms
                .get(PlatformType::Rust)
                .into_diagnostic()?
                .get_tool()
                .into_diagnostic()?;

            is_installed(*rust);
        }
    };

    Ok(())
}
