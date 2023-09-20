use moon_config::DenoConfig;
use moon_platform_runtime::RuntimeReq;
use moon_tool::{async_trait, Tool};
use proto_core::ProtoEnvironment;
use std::path::PathBuf;

#[derive(Debug)]
pub struct DenoTool {
    pub config: DenoConfig,

    pub global: bool,
}

impl DenoTool {
    pub fn new(
        _proto: &ProtoEnvironment,
        config: &DenoConfig,
        req: &RuntimeReq,
    ) -> miette::Result<DenoTool> {
        let mut deno = DenoTool {
            config: config.to_owned(),
            global: true,
        };

        if req.is_global() {
            deno.global = true;
            // node.config.version = None;
        } else {
            // node.config.version = Some(version.number.to_owned());
        };

        Ok(deno)
    }
}

#[async_trait]
impl Tool for DenoTool {
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
    }

    fn get_bin_path(&self) -> miette::Result<PathBuf> {
        Ok(PathBuf::from("deno"))
    }
}
