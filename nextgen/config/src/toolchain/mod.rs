mod bin_config;
mod bun_config;
mod deno_config;
mod node_config;
mod rust_config;
mod typescript_config;

pub use bin_config::*;
pub use bun_config::*;
pub use deno_config::*;
pub use node_config::*;
pub use rust_config::*;
pub use typescript_config::*;

#[cfg(feature = "loader")]
#[macro_export]
macro_rules! is_using_tool_version {
    ($self:ident, $parent_tool:ident, $tool:ident) => {
        if let Some(config) = &$self.$parent_tool {
            is_using_tool_version!(config, $tool);
        }
    };
    ($self:ident, $tool:ident) => {
        if let Some(config) = &$self.$tool {
            if config.version.is_some() {
                return true;
            }
        }
    };
}

#[cfg(feature = "proto")]
#[macro_export]
macro_rules! inherit_tool {
    ($config:ident, $tool:ident, $key:expr, $method:ident) => {
        pub fn $method(&mut self, proto_config: &proto_core::ProtoConfig) -> miette::Result<()> {
            if let Some(version) = proto_config.versions.get($key) {
                let config = self.$tool.get_or_insert_with($config::default);

                if config.version.is_none() {
                    config.version = Some(version.to_owned());
                }
            }

            if let Some(config) = &mut self.$tool {
                if config.plugin.is_none() {
                    config.plugin = proto_config.plugins.get($key).cloned();
                }
            }

            Ok(())
        }
    };
}

#[cfg(feature = "proto")]
#[macro_export]
macro_rules! inherit_tool_required {
    ($config:ident, $tool:ident, $key:expr, $method:ident) => {
        pub fn $method(&mut self, proto_config: &proto_core::ProtoConfig) -> miette::Result<()> {
            if let Some(version) = proto_config.versions.get($key) {
                if self.$tool.version.is_none() {
                    self.$tool.version = Some(version.to_owned());
                }
            }

            if self.$tool.plugin.is_none() {
                self.$tool.plugin = proto_config.plugins.get($key).cloned();
            }

            Ok(())
        }
    };
}

#[cfg(feature = "proto")]
#[macro_export]
macro_rules! inherit_tool_without_version {
    ($config:ident, $tool:ident, $key:expr, $method:ident) => {
        pub fn $method(&mut self, proto_config: &proto_core::ProtoConfig) -> miette::Result<()> {
            if self.$tool.is_none() && proto_config.versions.get($key).is_some() {
                self.$tool = Some($config::default());
            }

            // Not used yet!
            // if let Some(config) = self.$tool.as_mut() {
            //     if config.plugin.is_none() {
            //         config.plugin = proto_config.plugins.get($key).cloned();
            //     }
            // }

            Ok(())
        }
    };
}
