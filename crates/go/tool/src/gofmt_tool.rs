use moon_config::GoConfig;
use moon_logger::debug;
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_tool::{Tool, ToolError};
use proto::{
    async_trait, go::GoLanguage, Describable, Executable, Installable, Proto, Resolvable,
    Tool as ProtoTool,
};
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct GofmtTool {
    pub config: GoConfig,
    pub tool: GoLanguage,
    bin_path: PathBuf,
}

impl GofmtTool {
    pub fn new(proto: &Proto, cfg: &GoConfig, version: &str) -> Result<GofmtTool, ToolError> {
        let mut config = cfg.to_owned();
        config.version = Some(version.to_owned());
        let tool =  GoLanguage::new(proto);

        let mut bin_path = tool.get_bin_path()?.to_owned();
        bin_path.pop();
        bin_path.push("gofmt");

        Ok(GofmtTool {
            config,
            tool,
            bin_path,
        })
    }
}

#[async_trait]
impl Tool for GofmtTool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_bin_path(&self) -> Result<&Path, ToolError> {
        Ok(&self.bin_path)
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
