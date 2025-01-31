use moon_config::RustConfig;
use moon_console::{Checkpoint, Console};
use moon_logger::debug;
use moon_process::Command;
use moon_tool::{
    async_trait, get_proto_paths, load_tool_plugin, prepend_path_env_var, use_global_tool_on_path,
    Tool,
};
use moon_toolchain::RuntimeReq;
use proto_core::flow::install::InstallOptions;
use proto_core::{Id, ProtoEnvironment, Tool as ProtoTool, UnresolvedVersionSpec};
use rustc_hash::FxHashMap;
use starbase_utils::env::path_var;
use std::path::PathBuf;
use std::sync::Arc;
use std::{ffi::OsStr, path::Path};
use tracing::instrument;

pub fn get_rust_env_paths(proto_env: &ProtoEnvironment) -> Vec<PathBuf> {
    let mut paths = get_proto_paths(proto_env);

    if let Some(value) = path_var("CARGO_INSTALL_ROOT") {
        paths.push(value.join("bin"));
    }

    if let Some(value) = path_var("CARGO_HOME") {
        paths.push(value.join("bin"));
    }

    paths.push(proto_env.home_dir.join(".cargo").join("bin"));

    paths
}

pub struct RustTool {
    pub config: RustConfig,

    pub global: bool,

    pub tool: ProtoTool,

    console: Arc<Console>,

    proto_env: Arc<ProtoEnvironment>,
}

impl RustTool {
    pub async fn new(
        proto_env: Arc<ProtoEnvironment>,
        console: Arc<Console>,
        config: &RustConfig,
        req: &RuntimeReq,
    ) -> miette::Result<RustTool> {
        let mut rust = RustTool {
            config: config.to_owned(),
            global: false,
            tool: load_tool_plugin(
                &Id::raw("rust"),
                &proto_env,
                config.plugin.as_ref().unwrap(),
            )
            .await?,
            proto_env,
            console,
        };

        if use_global_tool_on_path("rust") || req.is_global() {
            rust.global = true;
            rust.config.version = None;
        } else {
            rust.config.version = req.to_spec();
        };

        Ok(rust)
    }

    #[instrument(skip_all)]
    pub async fn exec_cargo<I, S>(&self, args: I, working_dir: &Path) -> miette::Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Command::new("cargo")
            .args(args)
            .env(
                "PATH",
                prepend_path_env_var(get_rust_env_paths(&self.proto_env)),
            )
            .cwd(working_dir)
            .with_console(self.console.clone())
            .exec_stream_output()
            .await?;

        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn exec_rustup<I, S>(&self, args: I, working_dir: &Path) -> miette::Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Command::new("rustup")
            .args(args)
            .env(
                "PATH",
                prepend_path_env_var(get_rust_env_paths(&self.proto_env)),
            )
            .cwd(working_dir)
            .with_console(self.console.clone())
            .exec_stream_output()
            .await?;

        Ok(())
    }
}

#[async_trait]
impl Tool for RustTool {
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
    }

    #[instrument(skip_all)]
    async fn setup(
        &mut self,
        last_versions: &mut FxHashMap<String, UnresolvedVersionSpec>,
    ) -> miette::Result<u8> {
        let mut installed = 0;

        let Some(version) = &self.config.version else {
            return Ok(installed);
        };

        if self.global {
            debug!("Using global binary in PATH");
        } else if self.tool.is_setup(version).await? {
            debug!("Rust has already been setup");

            // When offline and the tool doesn't exist, fallback to the global binary
        } else if proto_core::is_offline() {
            debug!(
                "No internet connection and Rust has not been setup, falling back to global binary in PATH"
            );

            self.global = true;

            // Otherwise try and install the tool
        } else {
            let setup = match last_versions.get("rust") {
                Some(last) => version != last,
                None => true,
            };

            if setup || !self.tool.get_product_dir().exists() {
                self.console
                    .out
                    .print_checkpoint(Checkpoint::Setup, format!("installing rust {version}"))?;

                if self.tool.setup(version, InstallOptions::default()).await? {
                    last_versions.insert("rust".into(), version.to_owned());
                    installed += 1;
                }
            }
        }

        self.tool.locate_globals_dirs().await?;

        Ok(installed)
    }

    async fn teardown(&mut self) -> miette::Result<()> {
        self.tool.teardown().await?;

        Ok(())
    }
}
