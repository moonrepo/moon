mod deno_config;
mod node_config;
mod rust_config;
mod typescript_config;

pub use deno_config::*;
pub use node_config::*;
pub use rust_config::*;
pub use typescript_config::*;

#[macro_export]
macro_rules! inherit_tool {
    ($config:ident, $tool:ident, $key:expr, $method:ident) => {
        pub fn $method(&mut self, proto_tools: &ToolsConfig) -> Result<(), ConfigError> {
            if let Some(version) = proto_tools.tools.get($key) {
                if let Some(config) = &mut self.$tool {
                    if config.version.is_none() {
                        config.version = Some(version.to_owned());
                    }
                } else {
                    let mut data = $config::default_values(&())?;
                    data.version = Some(version.to_owned());

                    self.$tool = Some(data);
                }
            }

            Ok(())
        }
    };
}

#[macro_export]
macro_rules! inherit_tool_required {
    ($config:ident, $tool:ident, $key:expr, $method:ident) => {
        pub fn $method(&mut self, proto_tools: &ToolsConfig) -> Result<(), ConfigError> {
            if let Some(version) = proto_tools.tools.get($key) {
                if self.$tool.version.is_none() {
                    self.$tool.version = Some(version.to_owned());
                }
            }

            Ok(())
        }
    };
}

#[macro_export]
macro_rules! inherit_tool_without_version {
    ($config:ident, $tool:ident, $key:expr, $method:ident) => {
        pub fn $method(&mut self, proto_tools: &ToolsConfig) -> Result<(), ConfigError> {
            if self.$tool.is_none() && proto_tools.tools.get($key).is_some() {
                self.$tool = Some($config::default_values(&())?);
            }

            Ok(())
        }
    };
}
