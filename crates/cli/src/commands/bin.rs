use monolith_toolchain::Tool;
use monolith_workspace::Workspace;

pub struct BinOptions {}

pub async fn bin(
    workspace: &Workspace,
    _options: BinOptions,
    tool: &str,
) -> Result<(), std::io::Error> {
    let toolchain = &workspace.toolchain;

    let bin_path = match tool {
        "node" => Some(toolchain.get_node().get_bin_path()),
        "npm" => Some(toolchain.get_npm().get_bin_path()),
        "npx" => Some(toolchain.get_npx().get_bin_path()),
        "pnpm" => toolchain.get_pnpm().map(|tool| tool.get_bin_path()),
        "yarn" => toolchain.get_yarn().map(|tool| tool.get_bin_path()),
        _ => None,
    };

    if let Some(path) = bin_path {
        println!("{}", path.display());
    }

    Ok(())
}
