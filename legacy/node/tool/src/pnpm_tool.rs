use crate::get_node_env_paths;
use crate::node_tool::NodeTool;
use moon_config::PnpmConfig;
use moon_console::{Checkpoint, Console};
use moon_logger::debug;
use moon_node_lang::{pnpm, LockfileDependencyVersions};
use moon_process::Command;
use moon_tool::{
    async_trait, get_proto_env_vars, get_proto_version_env, load_tool_plugin, prepend_path_env_var,
    use_global_tool_on_path, DependencyManager, Tool,
};
use moon_utils::get_workspace_root;
use proto_core::flow::install::InstallOptions;
use proto_core::{
    Id, ProtoEnvironment, Tool as ProtoTool, UnresolvedVersionSpec, VersionReq, VersionSpec,
};
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::env;
use std::path::Path;
use std::sync::Arc;
use tracing::instrument;

pub struct PnpmTool {
    pub config: PnpmConfig,

    pub global: bool,

    pub tool: ProtoTool,

    console: Arc<Console>,

    proto_env: Arc<ProtoEnvironment>,
}

impl PnpmTool {
    pub async fn new(
        proto_env: Arc<ProtoEnvironment>,
        console: Arc<Console>,
        config: &Option<PnpmConfig>,
    ) -> miette::Result<PnpmTool> {
        let config = config.to_owned().unwrap_or_default();

        Ok(PnpmTool {
            global: use_global_tool_on_path("pnpm") || config.version.is_none(),
            tool: load_tool_plugin(
                &Id::raw("pnpm"),
                &proto_env,
                config.plugin.as_ref().unwrap(),
            )
            .await?,
            config,
            proto_env,
            console,
        })
    }
}

#[async_trait]
impl Tool for PnpmTool {
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
    }

    #[instrument(skip_all)]
    async fn setup(
        &mut self,
        last_versions: &mut FxHashMap<String, UnresolvedVersionSpec>,
    ) -> miette::Result<u8> {
        let mut count = 0;
        let version = self.config.version.as_ref();

        let Some(version) = version else {
            return Ok(count);
        };

        if self.global {
            debug!("Using global binary in PATH");

            return Ok(count);
        }

        if self.tool.is_setup(version).await? {
            self.tool.locate_globals_dirs().await?;

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
            if last == version && self.tool.get_product_dir().exists() {
                return Ok(count);
            }
        }

        self.console
            .out
            .print_checkpoint(Checkpoint::Setup, format!("installing pnpm {version}"))?;

        if self.tool.setup(version, InstallOptions::default()).await? {
            last_versions.insert("pnpm".into(), version.to_owned());
            count += 1;
        }

        self.tool.locate_globals_dirs().await?;

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
        let mut cmd = Command::new("pnpm");
        cmd.with_console(self.console.clone());
        cmd.envs(get_proto_env_vars());

        if !self.global {
            cmd.env(
                "PATH",
                prepend_path_env_var(get_node_env_paths(&self.proto_env)),
            );
        }

        if let Some(version) = get_proto_version_env(&self.tool) {
            cmd.env("PROTO_PNPM_VERSION", version);
        }

        if let Some(version) = get_proto_version_env(&node.tool) {
            cmd.env("PROTO_NODE_VERSION", version);
        }

        Ok(cmd)
    }

    #[instrument(skip_all)]
    async fn dedupe_dependencies(
        &self,
        node: &NodeTool,
        working_dir: &Path,
        log: bool,
    ) -> miette::Result<()> {
        let Some(version_spec) = self.config.version.as_ref() else {
            return Ok(());
        };

        if working_dir.join(self.get_lock_filename()).exists() {
            let version = match version_spec {
                UnresolvedVersionSpec::Semantic(v) => v.to_owned(),
                _ => match self.tool.get_resolved_version() {
                    VersionSpec::Semantic(v) => v,
                    _ => return Ok(()),
                },
            };

            // https://github.com/pnpm/pnpm/releases/tag/v7.26.0
            if VersionReq::parse(">=7.26.0").unwrap().matches(&version) {
                self.create_command(node)?
                    .arg("dedupe")
                    .cwd(working_dir)
                    .set_print_command(log)
                    .exec_capture_output()
                    .await?;

                return Ok(());
            } else {
                node.exec_package("pnpm-deduplicate", &["pnpm-deduplicate"], working_dir)
                    .await?;
            }
        }

        Ok(())
    }

    fn get_lock_filename(&self) -> String {
        String::from("pnpm-lock.yaml")
    }

    fn get_manifest_filename(&self) -> String {
        String::from("package.json")
    }

    #[instrument(skip_all)]
    async fn get_resolved_dependencies(
        &self,
        project_root: &Path,
    ) -> miette::Result<LockfileDependencyVersions> {
        let Some(lockfile_path) =
            fs::find_upwards_until("pnpm-lock.yaml", project_root, get_workspace_root())
        else {
            return Ok(FxHashMap::default());
        };

        Ok(pnpm::load_lockfile_dependencies(lockfile_path)?)
    }

    #[instrument(skip_all)]
    async fn install_dependencies(
        &self,
        node: &NodeTool,
        working_dir: &Path,
        log: bool,
    ) -> miette::Result<()> {
        let mut cmd = self.create_command(node)?;

        cmd.args(["install"])
            .args(&self.config.install_args)
            .cwd(working_dir)
            .set_print_command(log);

        if env::var("MOON_TEST_HIDE_INSTALL_OUTPUT").is_ok() {
            cmd.exec_capture_output().await?;
        } else {
            cmd.exec_stream_output().await?;
        }

        Ok(())
    }

    #[instrument(skip_all)]
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

        cmd.exec_stream_output().await?;

        Ok(())
    }
}
