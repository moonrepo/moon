use moon_config::BunConfig;
use moon_logger::debug;
use moon_platform_runtime::RuntimeReq;
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_tool::{async_trait, load_tool_plugin, use_global_tool_on_path, Tool};
use proto_core::{Id, ProtoEnvironment, Tool as ProtoTool, UnresolvedVersionSpec};
use rustc_hash::FxHashMap;
use std::path::PathBuf;

pub struct BunTool {
    pub config: BunConfig,

    pub global: bool,

    pub tool: ProtoTool,
}

impl BunTool {
    pub async fn new(
        proto: &ProtoEnvironment,
        config: &BunConfig,
        req: &RuntimeReq,
    ) -> miette::Result<BunTool> {
        let mut bun = BunTool {
            config: config.to_owned(),
            global: false,
            tool: load_tool_plugin(&Id::raw("bun"), proto, config.plugin.as_ref().unwrap()).await?,
        };

        if use_global_tool_on_path() || req.is_global() {
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

    fn get_bin_path(&self) -> miette::Result<PathBuf> {
        Ok(PathBuf::from("bun"))
    }

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
            debug!("Bun has already been setup");

            // When offline and the tool doesn't exist, fallback to the global binary
        } else if proto_core::is_offline() {
            debug!(
                "No internet connection and Bun has not been setup, falling back to global binary in PATH"
            );

            self.global = true;

            // Otherwise try and install the tool
        } else {
            let setup = match last_versions.get("bun") {
                Some(last) => version != last,
                None => true,
            };

            if setup || !self.tool.get_tool_dir().exists() {
                print_checkpoint(format!("installing bun {version}"), Checkpoint::Setup);

                if self.tool.setup(version, false).await? {
                    last_versions.insert("bun".into(), version.to_owned());
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
