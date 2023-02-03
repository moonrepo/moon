use moon_tool::{Tool, ToolError};
use std::path::Path;
use moon_logger::debug;
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_config::GoConfig;
use rustc_hash::FxHashMap;
use proto::{
    async_trait, go::GoLanguage, Describable, Executable, Installable, Proto, Resolvable,
    Tool as ProtoTool,
};

#[derive(Debug)]
pub struct GofmtTool {
   pub config: GoConfig,
   pub tool: GoLanguage,
}

impl GofmtTool {
    pub fn new(proto: &Proto, config: &GoConfig, version: &str) -> Result<GofmtTool, ToolError> {
        let mut cfg = config.to_owned();
        cfg.version = Some(version.to_owned());

        Ok(GofmtTool {
            config: cfg,
            tool: GoLanguage::new(proto),
        })
    }
}

#[async_trait]
impl Tool for GofmtTool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_bin_path(&self) -> Result<&Path, ToolError> {
        let mut gobin = self.tool.get_bin_path()?.to_owned();
        gobin.pop();
        gobin.push("gofmt");

        Ok(&gobin)
    }

    fn get_version(&self) -> &str {
        self.tool.get_resolved_version()
    }

    async fn setup(
        &mut self,
        last_versions: &mut FxHashMap<String, String>,
    ) -> Result<u8, ToolError> {
        let mut installed = 0;
        let version_clone = self.config.version.clone();

        let Some(version) = version_clone else {
            return Ok(installed);
        };

        if self.tool.is_setup(&version).await? {
            debug!(target: self.tool.get_log_target(), "Go has already been setup");
        } else {
            let setup = match last_versions.get("go") {
                Some(last) => &version != last,
                None => true,
            };

            if setup || !self.tool.get_install_dir()?.exists() {
                print_checkpoint(format!("installing go {}", version), Checkpoint::Setup);

                if self.tool.setup(&version).await? {
                    last_versions.insert("go".into(), version.to_string());
                    installed += 1;
                }
            }
        }

        Ok(installed)
    }
}
