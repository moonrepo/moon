mod bin_config;
mod bun_config;
mod deno_config;
mod moon_config;
mod node_config;
mod plugin_config;
mod python_config;
mod rust_config;

pub use bin_config::*;
pub use bun_config::*;
pub use deno_config::*;
pub use moon_config::*;
pub use node_config::*;
pub use plugin_config::*;
pub use python_config::*;
pub use rust_config::*;

#[cfg(feature = "proto")]
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
            use moon_common::color;
            use tracing::trace;

            if let Some(version) = proto_config.versions.get($key) {
                let config = self.$tool.get_or_insert_with($config::default);

                if config.version.is_none() {
                    trace!(
                        "Inheriting {} version {} from .prototools",
                        color::id($key),
                        version
                    );

                    config.version = Some(version.req.to_owned());
                }
            }

            if let Some(config) = &mut self.$tool {
                if config.plugin.is_none() {
                    config.plugin = proto_config.plugins.get($key).cloned();

                    if let Some(plugin) = &config.plugin {
                        trace!(
                            plugin = plugin.to_string(),
                            "Inheriting {} plugin from proto",
                            color::id($key),
                        );
                    }
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
            use moon_common::color;
            use tracing::trace;

            if let Some(version) = proto_config.versions.get($key) {
                if self.$tool.version.is_none() {
                    trace!(
                        "Inheriting {} version {} from .prototools",
                        color::id($key),
                        version
                    );

                    self.$tool.version = Some(version.req.to_owned());
                }
            }

            if self.$tool.plugin.is_none() {
                self.$tool.plugin = proto_config.plugins.get($key).cloned();

                if let Some(plugin) = &self.$tool.plugin {
                    trace!(
                        plugin = plugin.to_string(),
                        "Inheriting {} plugin from proto",
                        color::id($key),
                    );
                }
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
