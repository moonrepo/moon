use crate::node_tool::NodeTool;
use moon_config::PnpmConfig;
use moon_logger::debug;
use moon_node_lang::{pnpm, LockfileDependencyVersions, PNPM};
use moon_process::Command;
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_tool::{async_trait, get_path_env_var, load_tool_plugin, DependencyManager, Tool};
use moon_utils::is_ci;
use proto_core::{Id, ProtoEnvironment, Tool as ProtoTool, UnresolvedVersionSpec, VersionReq};
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::env;
use std::path::{Path, PathBuf};

pub struct PnpmTool {
    pub config: PnpmConfig,

    pub global: bool,

    pub tool: ProtoTool,
}

impl PnpmTool {
    pub async fn new(
        proto: &ProtoEnvironment,
        config: &Option<PnpmConfig>,
    ) -> miette::Result<PnpmTool> {
        let config = config.to_owned().unwrap_or_default();

        Ok(PnpmTool {
            global: config.version.is_none(),
            tool: load_tool_plugin(&Id::raw("pnpm"), proto, config.plugin.as_ref().unwrap())
                .await?,
            config,
        })
    }
}

#[async_trait]
impl Tool for PnpmTool {
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
    }

    fn get_bin_path(&self) -> miette::Result<PathBuf> {
        Ok(if self.global {
            "pnpm".into()
        } else {
            self.tool.get_bin_path()?.to_path_buf()
        })
    }

    fn get_shim_path(&self) -> Option<PathBuf> {
        self.tool.get_shim_path().map(|p| p.to_path_buf())
    }

    async fn setup(
        &mut self,
        last_versions: &mut FxHashMap<String, UnresolvedVersionSpec>,
    ) -> miette::Result<u8> {
        let mut count = 0;
        let version = self.config.version.as_ref();

        let Some(version) = version else {
            return Ok(count);
        };

        if self.tool.is_setup(version).await? {
            self.tool.locate_globals_dir().await?;

            debug!("pnpm has already been setup");

            return Ok(count);
        }

        // When offline and the tool doesn't exist, fallback to the global binary
        if proto_core::is_offline() {
            debug!(
                "No internet connection and pnpm has not been setup, falling back to global binary in PATH"
            );

            self.global = true;

            return Ok(count);
        }

        if let Some(last) = last_versions.get("pnpm") {
            if last == version && self.tool.get_tool_dir().exists() {
                return Ok(count);
            }
        }

        print_checkpoint(format!("installing pnpm {version}"), Checkpoint::Setup);

        if self.tool.setup(version).await? {
            last_versions.insert("pnpm".into(), version.to_owned());
            count += 1;
        }

        self.tool.locate_globals_dir().await?;

        Ok(count)
    }

    async fn teardown(&mut self) -> miette::Result<()> {
        self.tool.teardown().await?;

        Ok(())
    }
}

#[async_trait]
impl DependencyManager<NodeTool> for PnpmTool {
    fn create_command(&self, node: &NodeTool) -> miette::Result<Command> {
        let mut cmd = if self.global {
            Command::new("pnpm")
        } else if let Some(shim) = self.get_shim_path() {
            Command::new(shim)
        } else {
            let mut cmd = Command::new(node.get_bin_path()?);
            cmd.arg(self.get_bin_path()?);
            cmd
        };

        if !self.global {
            cmd.env("PATH", get_path_env_var(&self.tool.get_tool_dir()));
        }

        cmd.env("PROTO_NODE_BIN", node.get_bin_path()?);

        Ok(cmd)
    }

    async fn dedupe_dependencies(
        &self,
        node: &NodeTool,
        working_dir: &Path,
        log: bool,
    ) -> miette::Result<()> {
        let Some(version) = self.config.version.as_ref() else {
            return Ok(());
        };

        if working_dir.join(self.get_lock_filename()).exists() {
            // https://github.com/pnpm/pnpm/releases/tag/v7.26.0
            if let UnresolvedVersionSpec::Version(ver) = version {
                if VersionReq::parse(">=7.26.0").unwrap().matches(ver) {
                    self.create_command(node)?
                        .arg("dedupe")
                        .cwd(working_dir)
                        .set_print_command(log)
                        .create_async()
                        .exec_capture_output()
                        .await?;

                    return Ok(());
                }
            }

            node.exec_package("pnpm-deduplicate", &["pnpm-deduplicate"], working_dir)
                .await?;
        }

        Ok(())
    }

    fn get_lock_filename(&self) -> String {
        String::from(PNPM.lockfile)
    }

    fn get_manifest_filename(&self) -> String {
        String::from(PNPM.manifest)
    }

    async fn get_resolved_dependencies(
        &self,
        project_root: &Path,
    ) -> miette::Result<LockfileDependencyVersions> {
        let Some(lockfile_path) = fs::find_upwards(PNPM.lockfile, project_root) else {
            return Ok(FxHashMap::default());
        };

        Ok(pnpm::load_lockfile_dependencies(lockfile_path)?)
    }

    async fn install_dependencies(
        &self,
        node: &NodeTool,
        working_dir: &Path,
        log: bool,
    ) -> miette::Result<()> {
        let mut args = vec!["install"];

        if is_ci() {
            let lockfile = working_dir.join(self.get_lock_filename());

            // Will fail with "Headless installation requires a pnpm-lock.yaml file"
            if lockfile.exists() {
                args.push("--frozen-lockfile");
            }
        }

        let mut cmd = self.create_command(node)?;

        cmd.args(args).cwd(working_dir).set_print_command(log);

        let mut cmd = cmd.create_async();

        if env::var("MOON_TEST_HIDE_INSTALL_OUTPUT").is_ok() {
            cmd.exec_capture_output().await?;
        } else {
            cmd.exec_stream_output().await?;
        }

        Ok(())
    }

    async fn install_focused_dependencies(
        &self,
        node: &NodeTool,
        packages: &[String],
        production_only: bool,
    ) -> miette::Result<()> {
        let mut cmd = self.create_command(node)?;
        cmd.arg("install");

        if production_only {
            cmd.arg("--prod");
        }

        for package in packages {
            cmd.arg(if production_only {
                "--filter-prod"
            } else {
                "--filter"
            });

            // https://pnpm.io/filtering#--filter-package_name-1
            cmd.arg(format!("{package}..."));
        }

        cmd.create_async().exec_stream_output().await?;

        Ok(())
    }
}
