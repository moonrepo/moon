use moon_config::DenoConfig;
use moon_platform_runtime::RuntimeReq;
use moon_tool::{async_trait, get_proto_paths, use_global_tool_on_path, Tool};
use proto_core::ProtoEnvironment;
use std::env;
use std::path::PathBuf;

pub fn get_deno_env_paths(proto_env: &ProtoEnvironment) -> Vec<PathBuf> {
    let mut paths = get_proto_paths(proto_env);

    if let Ok(value) = env::var("DENO_INSTALL_ROOT") {
        paths.push(PathBuf::from(value).join("bin"));
    }

    if let Ok(value) = env::var("DENO_HOME") {
        paths.push(PathBuf::from(value).join("bin"));
    }

    paths.push(proto_env.home.join(".deno").join("bin"));

    paths
}

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

        if use_global_tool_on_path() || req.is_global() {
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
}
