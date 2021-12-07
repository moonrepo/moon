use crate::errors::ToolchainError;
use crate::helpers::exec_command;
use crate::tool::Tool;
use crate::Toolchain;
use std::env::consts;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct NpxTool {
    bin_path: PathBuf,
}

impl NpxTool {
    pub fn new(toolchain: &Toolchain) -> NpxTool {
        let mut bin_path = toolchain.get_node().get_install_dir().clone();

        if consts::OS == "windows" {
            bin_path.push("npx");
        } else {
            bin_path.push("bin/npx");
        }

        NpxTool { bin_path }
    }

    pub async fn exec_bin(
        &self,
        package: &str,
        args: Vec<&str>,
        cwd: &Path,
    ) -> Result<(), ToolchainError> {
        let mut exec_args = vec!["--package", package, "--"];

        exec_args.extend(args);

        exec_command(&self.bin_path, exec_args, cwd).await?;

        Ok(())
    }
}
