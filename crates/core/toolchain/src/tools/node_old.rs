use crate::errors::ToolchainError;
use crate::helpers::{download_file_from_url, get_file_sha256_hash, unpack};
use crate::pms::npm::NpmTool;
use crate::pms::pnpm::PnpmTool;
use crate::pms::yarn::YarnTool;
use crate::traits::{DependencyManager, Downloadable, Executable, Installable, Lifecycle, Tool};
use crate::{get_path_env_var, ToolchainPaths};
use async_trait::async_trait;
use moon_config::{NodeConfig, NodePackageManager};
use moon_error::map_io_to_fs_error;
use moon_lang::LangError;
use moon_logger::{color, debug, error, Logable};
use moon_node_lang::node;
use moon_utils::fs;
use moon_utils::process::Command;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

impl NodeTool {
    pub async fn exec_package(
        &self,
        package: &str,
        args: Vec<&str>,
        working_dir: &Path,
    ) -> Result<(), ToolchainError> {
        let mut exec_args = vec!["--silent", "--package", package, "--"];

        exec_args.extend(args);

        let npx_path = node::find_package_manager_bin(&self.install_dir, "npx");

        Command::new(&npx_path)
            .args(exec_args)
            .cwd(working_dir)
            .env("PATH", get_path_env_var(&self.install_dir))
            .exec_stream_output()
            .await?;

        Ok(())
    }

    pub fn find_package_bin(
        &self,
        starting_dir: &Path,
        bin_name: &str,
    ) -> Result<node::BinFile, ToolchainError> {
        match node::find_package_bin(starting_dir, bin_name)? {
            Some(bin) => Ok(bin),
            None => Err(ToolchainError::MissingNodeModuleBin(bin_name.to_owned())),
        }
    }
}

#[async_trait]
impl Lifecycle<()> for NodeTool {
    async fn setup(&mut self, _parent: &(), check_version: bool) -> Result<u8, ToolchainError> {
        let mut installed = 0;

        if self.npm.is_some() {
            let mut npm = self.npm.take().unwrap();
            installed += npm.run_setup(self, check_version).await?;
            self.npm = Some(npm);
        }

        if self.pnpm.is_some() {
            let mut pnpm = self.pnpm.take().unwrap();
            installed += pnpm.run_setup(self, check_version).await?;
            self.pnpm = Some(pnpm);
        }

        if self.yarn.is_some() {
            let mut yarn = self.yarn.take().unwrap();
            installed += yarn.run_setup(self, check_version).await?;
            self.yarn = Some(yarn);
        }

        Ok(installed)
    }
}
