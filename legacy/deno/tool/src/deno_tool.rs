use moon_config::DenoConfig;
use moon_console::{Checkpoint, Console};
use moon_deno_lang::{LockfileDependencyVersions, load_lockfile_dependencies};
use moon_process::Command;
use moon_tool::{
    DependencyManager, Tool, async_trait, get_proto_env_vars, get_proto_paths,
    get_proto_version_env, load_tool_plugin, prepend_path_env_var, use_global_tool_on_path,
};
use moon_toolchain::RuntimeReq;
use moon_utils::get_workspace_root;
use proto_core::flow::install::InstallOptions;
use proto_core::{Id, ProtoEnvironment, Tool as ProtoTool, UnresolvedVersionSpec};
use rustc_hash::FxHashMap;
use starbase_utils::env::path_var;
use starbase_utils::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, instrument};

pub fn get_deno_env_paths(proto_env: &ProtoEnvironment) -> Vec<PathBuf> {
    let mut paths = get_proto_paths(proto_env);

    if let Some(value) = path_var("DENO_INSTALL_ROOT") {
        paths.push(value.join("bin"));
    }

    if let Some(value) = path_var("DENO_HOME") {
        paths.push(value.join("bin"));
    }

    paths.push(proto_env.home_dir.join(".deno").join("bin"));

    paths
}

pub struct DenoTool {
    pub config: DenoConfig,

    pub global: bool,

    pub tool: ProtoTool,

    #[allow(dead_code)]
    console: Arc<Console>,

    proto_env: Arc<ProtoEnvironment>,
}

impl DenoTool {
    pub async fn new(
        proto: Arc<ProtoEnvironment>,
        console: Arc<Console>,
        config: &DenoConfig,
        req: &RuntimeReq,
    ) -> miette::Result<DenoTool> {
        let mut deno = DenoTool {
            config: config.to_owned(),
            global: false,
            console,
            tool: load_tool_plugin(&Id::raw("deno"), &proto, config.plugin.as_ref().unwrap())
                .await?,
            proto_env: proto,
        };

        if use_global_tool_on_path("deno") || req.is_global() || deno.config.version.is_none() {
            deno.global = true;
            deno.config.version = None;
        } else {
            deno.config.version = req.to_spec();
        };

        Ok(deno)
    }

    pub fn is_v2(&self) -> bool {
        self.config.version.as_ref().is_some_and(|spec| match spec {
            UnresolvedVersionSpec::Req(req) => req.comparators.iter().any(|c| c.major >= 2),
            UnresolvedVersionSpec::Semantic(version) => version.major >= 2,
            _ => false,
        })
    }
}

#[async_trait]
impl Tool for DenoTool {
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

            debug!("Deno has already been setup");

            return Ok(count);
        }

        // When offline and the tool doesn't exist, fallback to the global binary
        if proto_core::is_offline() {
            debug!(
                "No internet connection and Deno has not been setup, falling back to global binary in PATH"
            );

            self.global = true;

            return Ok(count);
        }

        if let Some(last) = last_versions.get("deno") {
            if last == version && self.tool.get_product_dir().exists() {
                return Ok(count);
            }
        }

        self.console
            .print_checkpoint(Checkpoint::Setup, format!("installing deno {version}"))?;

        if self.tool.setup(version, InstallOptions::default()).await? {
            last_versions.insert("deno".into(), version.to_owned());
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
impl DependencyManager<()> for DenoTool {
    fn create_command(&self, _parent: &()) -> miette::Result<Command> {
        let mut cmd = Command::new("deno");
        cmd.with_console(self.console.clone());
        cmd.envs(get_proto_env_vars());

        if !self.global {
            cmd.env(
                "PATH",
                prepend_path_env_var(get_deno_env_paths(&self.proto_env)),
            );
        }

        if let Some(version) = get_proto_version_env(&self.tool) {
            cmd.env("PROTO_DENO_VERSION", version);
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
        String::from("deno.lock")
    }

    fn get_manifest_filename(&self) -> String {
        String::from("deno.json")
    }

    #[instrument(skip_all)]
    async fn get_resolved_dependencies(
        &self,
        project_root: &Path,
    ) -> miette::Result<LockfileDependencyVersions> {
        let Some(lockfile_path) =
            fs::find_upwards_until("deno.lock", project_root, get_workspace_root())
        else {
            return Ok(FxHashMap::default());
        };

        Ok(load_lockfile_dependencies(lockfile_path)?)
    }

    #[instrument(skip_all)]
    async fn install_dependencies(
        &self,
        parent: &(),
        working_dir: &Path,
        log: bool,
    ) -> miette::Result<()> {
        let mut cmd = self.create_command(parent)?;

        if self.is_v2() {
            cmd.args(["install", "--no-check"]);
            cmd.args(&self.config.install_args);
        } else {
            cmd.args(["cache", "--lock-write", &self.config.deps_file]);
        }

        cmd.cwd(working_dir).set_print_command(log);

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
        _parent: &(),
        _package_names: &[String], // Not supporetd
        _production_only: bool,
    ) -> miette::Result<()> {
        Ok(())
    }
}
