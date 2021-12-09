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

pub async fn bin(workspace: &Workspace, tool: &BinTools) -> Result<(), std::io::Error> {
    let toolchain = &workspace.toolchain;

    match tool {
        BinTools::Node => {
            println!("{}", toolchain.get_node().get_bin_path().display());
        }
        BinTools::Npm => {
            println!("{}", toolchain.get_npm().get_bin_path().display());
        }
        BinTools::Pnpm => {
            if let Some(tool) = toolchain.get_pnpm() {
                println!("{}", tool.get_bin_path().display());
            }
        }
        BinTools::Yarn => {
            if let Some(tool) = toolchain.get_yarn() {
                println!("{}", tool.get_bin_path().display());
            }
        }
    };

    Ok(())
}
