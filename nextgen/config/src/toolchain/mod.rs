mod bin_config;
mod deno_config;
mod node_config;
mod rust_config;
mod typescript_config;

pub use bin_config::*;
pub use deno_config::*;
pub use node_config::*;
pub use rust_config::*;
pub use typescript_config::*;

use proto_core::UnresolvedVersionSpec;
use semver::Version;

pub fn extract_version_from_proto_config(version: &UnresolvedVersionSpec) -> Option<Version> {
    match version {
        UnresolvedVersionSpec::Version(ver) => Some(ver.to_owned()),
        _ => None,
    }
}

#[macro_export]
macro_rules! inherit_tool {
    ($config:ident, $tool:ident, $key:expr, $method:ident) => {
        pub fn $method(&mut self, proto_tools: &ToolsConfig) -> miette::Result<()> {
            if let Some(version) = proto_tools.tools.get($key) {
                let config = self.$tool.get_or_insert_with($config::default);

                if config.version.is_none() {
                    config.version = extract_version_from_proto_config(version);
                }
            }

            if let Some(config) = &mut self.$tool {
                if config.plugin.is_none() {
                    config.plugin = proto_tools.plugins.get($key).cloned();
                }
            }

            Ok(())
        }
    };
}

#[macro_export]
macro_rules! inherit_tool_required {
    ($config:ident, $tool:ident, $key:expr, $method:ident) => {
        pub fn $method(&mut self, proto_tools: &ToolsConfig) -> miette::Result<()> {
            if let Some(version) = proto_tools.tools.get($key) {
                if self.$tool.version.is_none() {
                    self.$tool.version = extract_version_from_proto_config(version);
                }
            }

            if self.$tool.plugin.is_none() {
                self.$tool.plugin = proto_tools.plugins.get($key).cloned();
            }

            Ok(())
        }
    };
}

#[macro_export]
macro_rules! inherit_tool_without_version {
    ($config:ident, $tool:ident, $key:expr, $method:ident) => {
        pub fn $method(&mut self, proto_tools: &ToolsConfig) -> miette::Result<()> {
            if self.$tool.is_none() && proto_tools.tools.get($key).is_some() {
                self.$tool = Some($config::default());
            }

            if let Some(config) = self.$tool.as_mut() {
                if config.plugin.is_none() {
                    config.plugin = proto_tools.plugins.get($key).cloned();
                }
            }

            Ok(())
        }
    };
}
