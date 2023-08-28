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

#[macro_export]
macro_rules! inherit_tool {
    ($config:ident, $tool:ident, $key:expr, $method:ident) => {
        pub fn $method(&mut self, proto_tools: &ToolsConfig) -> miette::Result<()> {
            if let Some(version) = proto_tools.tools.get($key) {
                let config = self.$tool.get_or_insert_with($config::default);

                if config.version.is_none() {
                    config.version = Some(version.to_string());
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
                    self.$tool.version = Some(version.to_string());
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
