use crate::bun_tool::BunTool;
use crate::npm_tool::NpmTool;
use crate::pnpm_tool::PnpmTool;
use crate::yarn_tool::YarnTool;
use moon_config::{NodeConfig, NodePackageManager, UnresolvedVersionSpec};
use moon_logger::debug;
use moon_node_lang::node;
use moon_platform_runtime::RuntimeReq;
use moon_process::Command;
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_tool::{
    async_trait, load_tool_plugin, prepend_path_env_var, use_global_tool_on_path,
    DependencyManager, Tool, ToolError,
};
use proto_core::{Id, ProtoEnvironment, Tool as ProtoTool};
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};

pub struct NodeTool {
    pub config: NodeConfig,

    pub global: bool,

    pub tool: ProtoTool,

    bun: Option<BunTool>,

    npm: Option<NpmTool>,

    pnpm: Option<PnpmTool>,

    yarn: Option<YarnTool>,
}

impl NodeTool {
    pub async fn new(
        proto: &ProtoEnvironment,
        config: &NodeConfig,
        req: &RuntimeReq,
    ) -> miette::Result<NodeTool> {
        let mut node = NodeTool {
            global: false,
            config: config.to_owned(),
            tool: load_tool_plugin(&Id::raw("node"), proto, config.plugin.as_ref().unwrap())
                .await?,
            bun: None,
            npm: None,
            pnpm: None,
            yarn: None,
        };

        if use_global_tool_on_path() || req.is_global() {
            node.global = true;
            node.config.version = None;
        } else {
            node.config.version = req.to_spec();
        };

        match config.package_manager {
            NodePackageManager::Bun => {
                node.bun = Some(BunTool::new(proto, &config.bun).await?);
            }
            NodePackageManager::Npm => {
                node.npm = Some(NpmTool::new(proto, &config.npm).await?);
            }
            NodePackageManager::Pnpm => {
                node.pnpm = Some(PnpmTool::new(proto, &config.pnpm).await?);
            }
            NodePackageManager::Yarn => {
                node.yarn = Some(YarnTool::new(proto, &config.yarn).await?);
            }
        };

        Ok(node)
    }

    pub async fn exec_package(
        &self,
        package: &str,
        args: &[&str],
        working_dir: &Path,
    ) -> miette::Result<()> {
        let mut cmd = match &self.config.package_manager {
            NodePackageManager::Bun => {
                let mut cmd = self.get_bun()?.create_command(self)?;
                cmd.args(["x", "--bun", package]);
                cmd
            }
            NodePackageManager::Pnpm => {
                let mut cmd = self.get_pnpm()?.create_command(self)?;
                cmd.args(["--silent", "dlx", package]);
                cmd
            }
            NodePackageManager::Yarn if self.get_yarn()?.is_berry() => {
                let mut cmd = self.get_yarn()?.create_command(self)?;
                cmd.args(["dlx", "--quiet", package]);
                cmd
            }
            // Fallthrough to npx
            _ => {
                let mut cmd = Command::new(self.get_npx_path()?);
                cmd.args(["--silent", "--", package]);

                if !self.global {
                    cmd.env(
                        "PATH",
                        prepend_path_env_var([self.tool.get_bin_path()?.parent().unwrap()]),
                    );
                }

                cmd
            }
        };

        cmd.args(args)
            .cwd(working_dir)
            .create_async()
            .exec_stream_output()
            .await?;

        Ok(())
    }

    /// Return the `bun` package manager.
    pub fn get_bun(&self) -> miette::Result<&BunTool> {
        match &self.bun {
            Some(bun) => Ok(bun),
            None => Err(ToolError::UnknownTool("bun".into()).into()),
        }
    }

    /// Return the `npm` package manager.
    pub fn get_npm(&self) -> miette::Result<&NpmTool> {
        match &self.npm {
            Some(npm) => Ok(npm),
            None => Err(ToolError::UnknownTool("npm".into()).into()),
        }
    }

    pub fn get_npx_path(&self) -> miette::Result<PathBuf> {
        if self.global {
            return Ok("npx".into());
        }

        Ok(node::find_package_manager_bin(
            self.tool.get_tool_dir(),
            "npx",
        ))
    }

    /// Return the `pnpm` package manager.
    pub fn get_pnpm(&self) -> miette::Result<&PnpmTool> {
        match &self.pnpm {
            Some(pnpm) => Ok(pnpm),
            None => Err(ToolError::UnknownTool("pnpm".into()).into()),
        }
    }

    /// Return the `yarn` package manager.
    pub fn get_yarn(&self) -> miette::Result<&YarnTool> {
        match &self.yarn {
            Some(yarn) => Ok(yarn),
            None => Err(ToolError::UnknownTool("yarn".into()).into()),
        }
    }

    pub fn get_package_manager(&self) -> &(dyn DependencyManager<Self> + Send + Sync) {
        if self.bun.is_some() {
            return self.get_bun().unwrap();
        }

        if self.pnpm.is_some() {
            return self.get_pnpm().unwrap();
        }

        if self.yarn.is_some() {
            return self.get_yarn().unwrap();
        }

        if self.npm.is_some() {
            return self.get_npm().unwrap();
        }

        panic!("No package manager, how's this possible?");
    }
}

#[async_trait]
impl Tool for NodeTool {
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
    }

    fn get_bin_path(&self) -> miette::Result<PathBuf> {
        Ok(if self.global {
            "node".into()
        } else {
            self.tool.get_bin_path()?.to_path_buf()
        })
    }

    async fn setup(
        &mut self,
        last_versions: &mut FxHashMap<String, UnresolvedVersionSpec>,
    ) -> miette::Result<u8> {
        let mut installed = 0;

        // Don't abort early, as we need to setup package managers below
        if let Some(version) = &self.config.version {
            if self.global {
                debug!("Using global binary in PATH");
            } else if self.tool.is_setup(version).await? {
                debug!("Node.js has already been setup");

                // When offline and the tool doesn't exist, fallback to the global binary
            } else if proto_core::is_offline() {
                debug!(
                    "No internet connection and Node.js has not been setup, falling back to global binary in PATH"
                );

                self.global = true;

                // Otherwise try and install the tool
            } else {
                let setup = match last_versions.get("node") {
                    Some(last) => version != last,
                    None => true,
                };

                if setup || !self.tool.get_tool_dir().exists() {
                    print_checkpoint(format!("installing node {version}"), Checkpoint::Setup);

                    if self.tool.setup(version, false).await? {
                        last_versions.insert("node".into(), version.to_owned());
                        installed += 1;
                    }
                }
            }
        }

        self.tool.locate_globals_dir().await?;

        if let Some(npm) = &mut self.npm {
            installed += npm.setup(last_versions).await?;
        }

        if let Some(bun) = &mut self.bun {
            installed += bun.setup(last_versions).await?;
        }

        if let Some(pnpm) = &mut self.pnpm {
            installed += pnpm.setup(last_versions).await?;
        }

        if self.yarn.is_some() {
            let mut yarn = self.yarn.take().unwrap();

            installed += yarn.setup(last_versions).await?;
            yarn.set_version(self).await?;

            self.yarn = Some(yarn);
        }

        Ok(installed)
    }

    async fn teardown(&mut self) -> miette::Result<()> {
        self.tool.teardown().await?;

        Ok(())
    }
}
