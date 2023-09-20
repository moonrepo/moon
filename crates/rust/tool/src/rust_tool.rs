use moon_config::RustConfig;
use moon_logger::debug;
use moon_platform_runtime2::RuntimeReq;
use moon_process::Command;
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_tool::{async_trait, load_tool_plugin, Tool};
use proto_core::{Id, ProtoEnvironment, Tool as ProtoTool, UnresolvedVersionSpec, Version};
use rustc_hash::FxHashMap;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

pub struct RustTool {
    pub config: RustConfig,

    pub global: bool,

    pub tool: ProtoTool,
}

impl RustTool {
    pub async fn new(
        proto: &ProtoEnvironment,
        config: &RustConfig,
        req: &RuntimeReq,
    ) -> miette::Result<RustTool> {
        let mut rust = RustTool {
            config: config.to_owned(),
            global: false,
            tool: load_tool_plugin(&Id::raw("rust"), proto, config.plugin.as_ref().unwrap())
                .await?,
        };

        if req.is_global() {
            rust.global = true;
            rust.config.version = None;
        } else {
            rust.config.version = req.to_version();
        };

        Ok(rust)
    }

    pub async fn exec_cargo<I, S>(&self, args: I, working_dir: &Path) -> miette::Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Command::new("cargo")
            .args(args)
            .cwd(working_dir)
            .create_async()
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

    fn get_bin_path(&self) -> miette::Result<PathBuf> {
        Ok(PathBuf::from("cargo"))
    }

    async fn setup(
        &mut self,
        last_versions: &mut FxHashMap<String, Version>,
    ) -> miette::Result<u8> {
        let mut installed = 0;

        let Some(version) = &self.config.version else {
            return Ok(installed);
        };

        let version_type = UnresolvedVersionSpec::Version(version.to_owned());

        if self.tool.is_setup(&version_type).await? {
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

            if setup || !self.tool.get_tool_dir().exists() {
                print_checkpoint(format!("installing rust v{version}"), Checkpoint::Setup);

                if self.tool.setup(&version_type).await? {
                    last_versions.insert("rust".into(), version.to_owned());
                    installed += 1;
                }
            }
        }

        self.tool.locate_globals_dir().await?;

        Ok(installed)
    }

    async fn teardown(&mut self) -> miette::Result<()> {
        self.tool.teardown().await?;

        Ok(())
    }
}
