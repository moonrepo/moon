use moon_bun_lang::{load_lockfile_dependencies, LockfileDependencyVersions};
use moon_config::BunConfig;
use moon_console::{Checkpoint, Console};
use moon_logger::debug;
use moon_platform_runtime::RuntimeReq;
use moon_process::{output_to_string, Command};
use moon_tool::{
    async_trait, get_proto_env_vars, get_proto_paths, get_proto_version_env, load_tool_plugin,
    prepend_path_env_var, use_global_tool_on_path, DependencyManager, Tool,
};
use moon_utils::get_workspace_root;
use proto_core::{Id, ProtoEnvironment, Tool as ProtoTool, UnresolvedVersionSpec};
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn get_bun_env_paths(proto_env: &ProtoEnvironment) -> Vec<PathBuf> {
    let mut paths = get_proto_paths(proto_env);
    paths.push(proto_env.home.join(".bun").join("install").join("global"));
    paths.push(proto_env.home.join(".bun").join("bin"));
    paths
}

pub struct BunTool {
    pub config: BunConfig,

    pub global: bool,

    pub tool: ProtoTool,

    console: Arc<Console>,

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
            proto_env: proto,
        };

        if use_global_tool_on_path() || req.is_global() || bun.config.version.is_none() {
            bun.global = true;
            bun.config.version = None;
        } else {
            bun.config.version = req.to_spec();
        };

        Ok(bun)
    }
}

#[async_trait]
impl Tool for BunTool {
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
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

        if self.global {
            debug!("Using global binary in PATH");

            return Ok(count);
        }

        if self.tool.is_setup(version).await? {
            self.tool.locate_globals_dir().await?;

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
            if last == version && self.tool.get_tool_dir().exists() {
                return Ok(count);
            }
        }

        self.console
            .out
            .print_checkpoint(Checkpoint::Setup, format!("installing bun {version}"))?;

        if self.tool.setup(version, false).await? {
            last_versions.insert("bun".into(), version.to_owned());
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
        }

        // Tell proto to resolve instead of failing
        cmd.env_if_missing("PROTO_BUN_VERSION", "*");

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
        String::from("bun.lockb")
    }

    fn get_manifest_filename(&self) -> String {
        String::from("package.json")
    }

    async fn get_resolved_dependencies(
        &self,
        project_root: &Path,
    ) -> miette::Result<LockfileDependencyVersions> {
        let Some(lockfile_path) =
            fs::find_upwards_until("bun.lockb", project_root, get_workspace_root())
        else {
            return Ok(FxHashMap::default());
        };

        // Bun lockfiles are binary, so we need to convert them to text first
        // using Bun itself!
        let mut cmd = self.create_command(&())?;
        cmd.arg("bun.lockb");
        cmd.cwd(lockfile_path.parent().unwrap());

        let output = cmd.create_async().exec_capture_output().await?;

        Ok(load_lockfile_dependencies(output_to_string(
            &output.stdout,
        ))?)
    }

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
        parent: &(),
        _package_names: &[String], // Not supporetd
        production_only: bool,
    ) -> miette::Result<()> {
        let mut cmd = self.create_command(parent)?;
        cmd.args(["install"]);

        if production_only {
            cmd.arg("--production");
        }

        cmd.create_async().exec_stream_output().await?;

        Ok(())
    }
}
