// .moon/toolchain.yml

use crate::toolchain::{DenoConfig, NodeConfig, RustConfig, TypeScriptConfig};
use schematic::{validate, Config};

/// Docs: https://moonrepo.dev/docs/config/toolchain
#[derive(Config)]
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
