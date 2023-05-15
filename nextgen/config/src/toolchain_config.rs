use crate::toolchain::{DenoConfig, NodeConfig, RustConfig, TypeScriptConfig};
use schematic::Config;

#[derive(Config)]
pub struct ToolchainConfig {
    // TODO validate
    #[setting(extend)]
    pub extends: Option<String>,

    #[setting(nested)]
    pub deno: Option<DenoConfig>,

    #[setting(nested)]
    pub node: Option<NodeConfig>,

    #[setting(nested)]
    pub rust: Option<RustConfig>,

    #[setting(nested)]
    pub typescript: Option<TypeScriptConfig>,

    #[setting(rename = "$schema")]
    pub schema: String,
}
