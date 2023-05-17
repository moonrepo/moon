// .moon/toolchain.yml

use crate::toolchain::*;
use crate::{inherit_tool, inherit_tool_without_version};
use proto::ToolsConfig;
use schematic::{validate, Config, ConfigError, ConfigLoader};
use std::path::Path;

/// Docs: https://moonrepo.dev/docs/config/toolchain
#[derive(Config)]
#[config(file = ".moon/toolchain.yml")]
pub struct ToolchainConfig {
    #[setting(
        default = "https://moonrepo.dev/schemas/toolchain.json",
        rename = "$schema"
    )]
    pub schema: String,

    #[setting(extend, validate = validate::extends_string)]
    pub extends: Option<String>,

    #[setting(nested)]
    pub deno: Option<DenoConfig>,

    #[setting(nested)]
    pub node: Option<NodeConfig>,

    #[setting(nested)]
    pub rust: Option<RustConfig>,

    #[setting(nested)]
    pub typescript: Option<TypeScriptConfig>,
}

impl ToolchainConfig {
    inherit_tool_without_version!(DenoConfig, deno, "deno", inherit_proto_deno);

    inherit_tool!(RustConfig, rust, "rust", inherit_proto_rust);

    inherit_tool!(NodeConfig, node, "node", inherit_proto_node);

    inherit_tool_without_version!(
        TypeScriptConfig,
        typescript,
        "typescript",
        inherit_proto_typescript
    );

    pub fn inherit_proto(&mut self, proto_tools: &ToolsConfig) -> Result<(), ConfigError> {
        self.inherit_proto_deno(proto_tools)?;
        self.inherit_proto_rust(proto_tools)?;
        self.inherit_proto_node(proto_tools)?;
        self.inherit_proto_typescript(proto_tools)?;

        if let Some(node_config) = &mut self.node {
            node_config.inherit_proto(proto_tools)?;
        }

        Ok(())
    }

    pub fn load<P: AsRef<Path>>(
        path: P,
        proto_tools: &ToolsConfig,
    ) -> Result<ToolchainConfig, ConfigError> {
        let mut result = ConfigLoader::<ToolchainConfig>::yaml()
            .file(path.as_ref())?
            .load()?;

        result.config.inherit_proto(proto_tools)?;

        Ok(result.config)
    }
}
