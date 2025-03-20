use crate::get_node_env_paths;
use crate::node_tool::NodeTool;
use moon_config::BunpmConfig;
use moon_console::{Checkpoint, Console};
use moon_logger::debug;
use moon_node_lang::{LockfileDependencyVersions, bun};
use moon_process::{Command, output_to_string};
use moon_tool::{
    DependencyManager, Tool, async_trait, get_proto_env_vars, get_proto_version_env,
    get_shared_lock, load_tool_plugin, prepend_path_env_var, use_global_tool_on_path,
};
use moon_utils::get_workspace_root;
use proto_core::flow::install::InstallOptions;
use proto_core::{Id, ProtoEnvironment, Tool as ProtoTool, UnresolvedVersionSpec};
use rustc_hash::FxHashMap;
use scc::hash_cache::Entry;
use starbase_utils::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::instrument;

pub struct BunTool {
    pub config: BunpmConfig,

    pub global: bool,

    pub tool: ProtoTool,

    console: Arc<Console>,

    lockfile_cache: scc::HashCache<PathBuf, Arc<String>>,

    proto_env: Arc<ProtoEnvironment>,
}

impl BunTool {
    pub async fn new(
        proto_env: Arc<ProtoEnvironment>,
        console: Arc<Console>,
        config: &Option<BunpmConfig>,
    ) -> miette::Result<BunTool> {
        let config = config.to_owned().unwrap_or_default();

        Ok(BunTool {
            global: use_global_tool_on_path("bun") || config.version.is_none(),
            tool: load_tool_plugin(&Id::raw("bun"), &proto_env, config.plugin.as_ref().unwrap())
                .await?,
            config,
            proto_env,
            lockfile_cache: scc::HashCache::default(),
            console,
        })
    }

    fn internal_create_command(&self) -> miette::Result<Command> {
        let mut cmd = Command::new("bun");
        cmd.with_console(self.console.clone());
        cmd.envs(get_proto_env_vars());

        if !self.global {
            cmd.env(
                "PATH",
                prepend_path_env_var(get_node_env_paths(&self.proto_env)),
            );
        }

        if let Some(version) = get_proto_version_env(&self.tool) {
            cmd.env("PROTO_BUN_VERSION", version);
        }

        Ok(cmd)
    }

    // Bun lockfiles are binary, so we need to convert them to text first
    // using Bun itself!
    async fn load_lockfile(&self, cwd: &Path) -> miette::Result<Arc<String>> {
        let key = cwd.to_path_buf();

        let cache = match self.lockfile_cache.entry_async(key).await {
            Entry::Occupied(o) => o.get().clone(),
            Entry::Vacant(v) => {
                let yarn_lock = cwd.join("yarn.lock");

                let content = if yarn_lock.exists() {
                    Arc::new(fs::read_file(yarn_lock)?)
                } else {
                    let mut cmd = self.internal_create_command()?;
                    cmd.arg("bun.lockb");
                    cmd.cwd(cwd);

                    let output = cmd.exec_capture_output().await?;

                    Arc::new(output_to_string(&output.stdout))
                };

                v.put_entry(content.clone());

                content
            }
        };

        Ok(cache)
    }
}

#[async_trait]
impl Tool for BunTool {
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

        // Don't collide with the bun platform!
        let mutex = get_shared_lock("bun_tool").await;
        let _lock = mutex.lock().await;

        if self.tool.is_setup(version).await? {
            self.tool.locate_globals_dirs().await?;

            debug!("bun has already been setup");

            return Ok(count);
        }

        // When offline and the tool doesn't exist, fallback to the global binary
        if proto_core::is_offline() {
            debug!(
                "No internet connection and bun has not been setup, falling back to global binary in PATH"
            );

            self.global = true;

            return Ok(count);
        }

        if let Some(last) = last_versions.get("bun") {
            if last == version && self.tool.get_product_dir().exists() {
                return Ok(count);
            }
        }

        self.console
            .print_checkpoint(Checkpoint::Setup, format!("installing bun {version}"))?;

        if self.tool.setup(version, InstallOptions::default()).await? {
            last_versions.insert("bun".into(), version.to_owned());
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
impl DependencyManager<NodeTool> for BunTool {
    fn create_command(&self, node: &NodeTool) -> miette::Result<Command> {
        let mut cmd = self.internal_create_command()?;

        if !self.global {
            cmd.env(
                "PATH",
                prepend_path_env_var(get_node_env_paths(&self.proto_env)),
            );
        }

        if let Some(version) = get_proto_version_env(&self.tool) {
            cmd.env("PROTO_BUN_VERSION", version);
            cmd.env("PROTO_NODE_VERSION", "*");
        }

        if let Some(version) = get_proto_version_env(&node.tool) {
            cmd.env("PROTO_NODE_VERSION", version);
        }

        Ok(cmd)
    }

    async fn dedupe_dependencies(
        &self,
        _node: &NodeTool,
        _working_dir: &Path,
        _log: bool,
    ) -> miette::Result<()> {
        // Not supported!

        Ok(())
    }

    fn get_lock_filename(&self) -> String {
        String::from(
            if self
                .config
                .install_args
                .iter()
                .any(|arg| arg == "--save-text-lockfile")
            {
                "bun.lock"
            } else {
                "bun.lockb"
            },
        )
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
            fs::find_upwards_until("bun.lockb", project_root, get_workspace_root())
        else {
            return Ok(FxHashMap::default());
        };

        Ok(bun::load_lockfile_dependencies(
            self.load_lockfile(lockfile_path.parent().unwrap()).await?,
            lockfile_path,
        )?)
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
        _package_names: &[String], // Not supporetd
        _production_only: bool,
    ) -> miette::Result<()> {
        let mut cmd = self.create_command(node)?;
        cmd.args(["install"]);

        // NOTE: This seems to *not* install any dependencies
        // if production_only {
        //     cmd.arg("--production");
        // }

        cmd.exec_stream_output().await?;

        Ok(())
    }
}
