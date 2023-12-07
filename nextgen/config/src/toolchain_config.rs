// .moon/toolchain.yml

use crate::language_platform::PlatformType;
use crate::toolchain::*;
use crate::validate::check_yml_extension;
use crate::{inherit_tool, inherit_tool_without_version};
use moon_common::consts;
use proto_core::ProtoConfig;
use schematic::{validate, Config, ConfigLoader};
use std::path::Path;

/// Docs: https://moonrepo.dev/docs/config/toolchain
#[derive(Clone, Config, Debug)]
pub struct ToolchainConfig {
    #[setting(
        default = "https://moonrepo.dev/schemas/toolchain.json",
        rename = "$schema"
    )]
    pub schema: String,

    #[setting(extend, validate = validate::extends_string)]
    pub extends: Option<String>,

    #[setting(nested)]
    pub bun: Option<BunConfig>,

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
    inherit_tool!(BunConfig, bun, "bun", inherit_proto_bun);

    inherit_tool_without_version!(DenoConfig, deno, "deno", inherit_proto_deno);

    inherit_tool!(NodeConfig, node, "node", inherit_proto_node);

    inherit_tool!(RustConfig, rust, "rust", inherit_proto_rust);

    inherit_tool_without_version!(
        TypeScriptConfig,
        typescript,
        "typescript",
        inherit_proto_typescript
    );

    pub fn get_enabled_platforms(&self) -> Vec<PlatformType> {
        let mut tools = vec![];

        if self.bun.is_some() {
            tools.push(PlatformType::Bun);
        }

        if self.deno.is_some() {
            tools.push(PlatformType::Deno);
        }

        if self.node.is_some() {
            tools.push(PlatformType::Node);
        }

        if self.rust.is_some() {
            tools.push(PlatformType::Rust);
        }

        tools
    }

    pub fn inherit_proto(&mut self, proto_config: &ProtoConfig) -> miette::Result<()> {
        self.inherit_proto_bun(proto_config)?;
        self.inherit_proto_deno(proto_config)?;
        self.inherit_proto_node(proto_config)?;
        self.inherit_proto_rust(proto_config)?;
        self.inherit_proto_typescript(proto_config)?;

        if let Some(node_config) = &mut self.node {
            node_config.inherit_proto(proto_config)?;
        }

        Ok(())
    }

    pub fn load<R: AsRef<Path>, P: AsRef<Path>>(
        workspace_root: R,
        path: P,
        proto_config: &ProtoConfig,
    ) -> miette::Result<ToolchainConfig> {
        let mut result = ConfigLoader::<ToolchainConfig>::new()
            .set_root(workspace_root)
            .file_optional(check_yml_extension(path.as_ref()))?
            .load()?;

        result.config.inherit_proto(proto_config)?;

        Ok(result.config)
    }

    pub fn load_from<R: AsRef<Path>>(
        workspace_root: R,
        proto_config: &ProtoConfig,
    ) -> miette::Result<ToolchainConfig> {
        let workspace_root = workspace_root.as_ref();

        Self::load(
            workspace_root,
            workspace_root
                .join(consts::CONFIG_DIRNAME)
                .join(consts::CONFIG_TOOLCHAIN_FILENAME),
            proto_config,
        )
    }
}
