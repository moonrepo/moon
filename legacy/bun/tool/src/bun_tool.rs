use moon_bun_lang::{LockfileDependencyVersions, load_lockfile_dependencies};
use moon_config::BunConfig;
use moon_console::{Checkpoint, Console};
use moon_logger::debug;
use moon_process::{Command, output_to_string};
use moon_tool::{
    DependencyManager, Tool, async_trait, get_proto_env_vars, get_proto_paths,
    get_proto_version_env, get_shared_lock, load_tool_plugin, prepend_path_env_var,
    use_global_tool_on_path,
};
use moon_toolchain::RuntimeReq;
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

pub fn get_bun_env_paths(proto_env: &ProtoEnvironment) -> Vec<PathBuf> {
    let mut paths = get_proto_paths(proto_env);
    paths.push(
        proto_env
            .home_dir
            .join(".bun")
            .join("install")
            .join("global"),
    );
    paths.push(proto_env.home_dir.join(".bun").join("bin"));
    paths
}

pub struct BunTool {
    pub config: BunConfig,

    pub global: bool,

    pub tool: ProtoTool,

    console: Arc<Console>,

    lockfile_cache: scc::HashCache<PathBuf, Arc<String>>,

    proto_env: Arc<ProtoEnvironment>,
}

impl BunTool {
    pub async fn new(
        proto: Arc<ProtoEnvironment>,
        console: Arc<Console>,
        config: &BunConfig,
        req: &RuntimeReq,
    ) -> miette::Result<BunTool> {
        let mut bun = BunTool {
            console,
            config: config.to_owned(),
            global: false,
            tool: load_tool_plugin(&Id::raw("bun"), &proto, config.plugin.as_ref().unwrap())
                .await?,
            lockfile_cache: scc::HashCache::default(),
            proto_env: proto,
        };

        if use_global_tool_on_path("bun") || req.is_global() || bun.config.version.is_none() {
            bun.global = true;
            bun.config.version = None;
        } else {
            bun.config.version = req.to_spec();
        };

        Ok(bun)
    }

    // Bun lockfiles are binary, so we need to convert them to text first
    // using Bun itself!
    async fn load_lockfile(&self, cwd: &Path) -> miette::Result<Arc<String>> {
        let key = cwd.to_path_buf();

        let cache = match self.lockfile_cache.entry_async(key).await {
            Entry::Occupied(o) => o.get().clone(),
            Entry::Vacant(v) => {
                let bun_lock = cwd.join("bun.lock");
                let yarn_lock = cwd.join("yarn.lock");

                let content = if bun_lock.exists() {
                    Arc::new(fs::read_file(bun_lock)?)
                } else if yarn_lock.exists() {
                    Arc::new(fs::read_file(yarn_lock)?)
                } else {
                    let mut cmd = self.create_command(&())?;
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

        // Don't collide with the node bun package manager!
        let mutex = get_shared_lock("bun_tool").await;
        let _lock = mutex.lock().await;

        if self.tool.is_setup(version).await? {
            self.tool.locate_globals_dirs().await?;

            debug!("Bun has already been setup");

            return Ok(count);
        }

        // When offline and the tool doesn't exist, fallback to the global binary
        if proto_core::is_offline() {
            debug!(
                "No internet connection and Bun has not been setup, falling back to global binary in PATH"
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
impl DependencyManager<()> for BunTool {
    fn create_command(&self, _parent: &()) -> miette::Result<Command> {
        let mut cmd = Command::new("bun");
        cmd.with_console(self.console.clone());
        cmd.envs(get_proto_env_vars());

        if !self.global {
            cmd.env(
                "PATH",
                prepend_path_env_var(get_bun_env_paths(&self.proto_env)),
            );
        }

        if let Some(version) = get_proto_version_env(&self.tool) {
            cmd.env("PROTO_BUN_VERSION", version);
            cmd.env("PROTO_NODE_VERSION", "*");
        }

        Ok(cmd)
    }

    async fn dedupe_dependencies(
        &self,
        _parent: &(),
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
        let mut lockfile_path =
            fs::find_upwards_until("bun.lockb", project_root, get_workspace_root());

        if lockfile_path.is_none() {
            lockfile_path = fs::find_upwards_until("bun.lock", project_root, get_workspace_root());
        }

        if let Some(lockfile_path) = lockfile_path {
            return load_lockfile_dependencies(
                self.load_lockfile(lockfile_path.parent().unwrap()).await?,
                lockfile_path,
            );
        }

        Ok(FxHashMap::default())
    }

    #[instrument(skip_all)]
    async fn install_dependencies(
        &self,
        parent: &(),
        working_dir: &Path,
        log: bool,
    ) -> miette::Result<()> {
        let mut cmd = self.create_command(parent)?;

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
        parent: &(),
        _package_names: &[String], // Not supporetd
        production_only: bool,
    ) -> miette::Result<()> {
        let mut cmd = self.create_command(parent)?;
        cmd.args(["install"]);

        if production_only {
            cmd.arg("--production");
        }

        cmd.exec_stream_output().await?;

        Ok(())
    }
}
