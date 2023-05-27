use moon_config2::RustConfig;
use moon_logger::debug;
use moon_platform_runtime::Version;
use moon_process::Command;
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_tool::{Tool, ToolError};
use proto::{async_trait, rust::RustLanguage, Installable, Proto, Tool as ProtoTool};
use rustc_hash::FxHashMap;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct RustTool {
    pub config: RustConfig,

    pub global: bool,

    pub tool: RustLanguage,
}

impl RustTool {
    pub fn new(
        proto: &Proto,
        config: &RustConfig,
        version: &Version,
    ) -> Result<RustTool, ToolError> {
        let mut rust = RustTool {
            config: config.to_owned(),
            global: false,
            tool: RustLanguage::new(proto),
        };

        if version.is_global() {
            rust.global = true;
            rust.config.version = None;
        } else {
            rust.config.version = Some(version.number.to_owned());
        };

        Ok(rust)
    }

    pub async fn exec_cargo<I, S>(&self, args: I, working_dir: &Path) -> Result<(), ToolError>
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
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_bin_path(&self) -> Result<PathBuf, ToolError> {
        Ok(PathBuf::from("cargo"))
    }

    async fn setup(
        &mut self,
        last_versions: &mut FxHashMap<String, String>,
    ) -> Result<u8, ToolError> {
        let mut installed = 0;

        let Some(version) = &self.config.version else  {
            return Ok(installed);
        };

        if self.tool.is_setup(version).await? {
            debug!("Rust has already been setup");

            // When offline and the tool doesn't exist, fallback to the global binary
        } else if proto::is_offline() {
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

            if setup || !self.tool.get_install_dir()?.exists() {
                print_checkpoint(format!("installing rust v{version}"), Checkpoint::Setup);

                if self.tool.setup(version).await? {
                    last_versions.insert("rust".into(), version.to_string());
                    installed += 1;
                }
            }
        }

        Ok(installed)
    }

    async fn teardown(&mut self) -> Result<(), ToolError> {
        self.tool.teardown().await?;

        Ok(())
    }
}
