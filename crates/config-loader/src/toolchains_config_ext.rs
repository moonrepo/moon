use moon_common::Id;
use moon_common::color;
use moon_config::{ToolchainPluginVersionFrom, ToolchainsConfig};
use proto_core::{
    PluginLocator, ProtoConfig, ToolContext, UnresolvedVersionSpec,
    warpgate::{DataLocator, find_debug_locator, find_debug_locator_with_url_fallback as locate},
};
use schematic::{ConfigError, Path, PathSegment, ValidateError, ValidatorError};
use tracing::trace;

pub struct ToolchainsConfigExt;

impl ToolchainsConfigExt {
    pub fn get_plugin_locator(id: &Id) -> Option<PluginLocator> {
        match id.as_str() {
            "bun" => Some(locate("bun_toolchain", "1.0.2")),
            "deno" => Some(locate("deno_toolchain", "1.0.3")),
            "go" => Some(locate("go_toolchain", "1.2.0")),
            "javascript" => Some(locate("javascript_toolchain", "1.0.7")),
            "node" => Some(locate("node_toolchain", "1.0.2")),
            "npm" => Some(locate("node_depman_toolchain", "1.0.3")),
            "pnpm" => Some(locate("node_depman_toolchain", "1.0.3")),
            "rust" => Some(locate("rust_toolchain", "1.0.5")),
            "typescript" => Some(locate("typescript_toolchain", "1.1.1")),
            "unstable_python" => Some(locate("python_toolchain", "0.1.6")),
            "unstable_pip" => Some(locate("python_pip_toolchain", "0.1.2")),
            "unstable_uv" => Some(locate("python_uv_toolchain", "0.1.2")),
            "yarn" => Some(locate("node_depman_toolchain", "1.0.3")),
            "system" => Some(PluginLocator::Data(Box::new(DataLocator {
                data: "data://system_toolchain".into(),
                bytes: Some(include_bytes!("../res/system_toolchain.wasm").to_vec()),
            }))),
            _ => None,
        }
    }

    pub fn inherit_defaults(
        config: &mut ToolchainsConfig,
        proto_config: &ProtoConfig,
    ) -> miette::Result<()> {
        Self::inherit_proto_versions_for_plugins(config, proto_config)?;
        Self::inherit_default_plugins(config);
        Self::inherit_plugin_locators(config)?;

        Ok(())
    }

    pub fn inherit_proto_versions_for_plugins(
        config: &mut ToolchainsConfig,
        proto_config: &ProtoConfig,
    ) -> miette::Result<()> {
        for (id, config) in &mut config.plugins {
            if config.version.is_some() {
                continue;
            }

            let proto_id = match &config.version_from_prototools {
                ToolchainPluginVersionFrom::Enabled(enabled) => {
                    if *enabled {
                        id.as_str().strip_prefix("unstable_").unwrap_or(id.as_str())
                    } else {
                        continue;
                    }
                }
                ToolchainPluginVersionFrom::Id(custom_id) => custom_id,
            };
            let proto_context = ToolContext::parse(proto_id).unwrap();

            if let Some(version) = proto_config.versions.get(&proto_context) {
                trace!(
                    "Inheriting {} version {} from .prototools",
                    color::id(id),
                    version
                );

                config.version = Some(version.req.to_owned());
            }
        }

        Ok(())
    }

    pub fn inherit_default_plugins(config: &mut ToolchainsConfig) {
        config.plugins.entry(Id::raw("system")).or_default();
    }

    pub fn inherit_test_plugins(config: &mut ToolchainsConfig) -> miette::Result<()> {
        for id in [
            "tc-tier1",
            "tc-tier2",
            "tc-tier2-reqs",
            "tc-tier2-setup-env",
            "tc-tier3",
            "tc-tier3-reqs",
        ] {
            config.plugins.entry(Id::raw(id)).or_default();
        }

        Ok(())
    }

    pub fn inherit_test_builtin_plugins(config: &mut ToolchainsConfig) -> miette::Result<()> {
        // We don't need all package managers
        for id in [
            "bun",
            "deno",
            "go",
            "javascript",
            "node",
            "npm",
            "rust",
            "system",
            "typescript",
            "unstable_python",
            "unstable_pip",
        ] {
            config.plugins.entry(Id::raw(id)).or_default();
        }

        Ok(())
    }

    pub fn inherit_plugin_locators(config: &mut ToolchainsConfig) -> miette::Result<()> {
        for (id, config) in config.plugins.iter_mut() {
            if config.plugin.is_some() {
                continue;
            }

            match id.as_str() {
                "bun" | "deno" | "go" | "javascript" | "node" | "npm" | "pnpm" | "rust"
                | "system" | "typescript" | "unstable_python" | "unstable_pip" | "unstable_uv"
                | "yarn" => {
                    config.plugin = Self::get_plugin_locator(id);
                }
                #[cfg(debug_assertions)]
                "tc-tier1" | "tc-tier2" | "tc-tier2-reqs" | "tc-tier2-setup-env" | "tc-tier3"
                | "tc-tier3-reqs" => {
                    config.plugin = Some(
                        find_debug_locator(&id.replace("-", "_"))
                            .expect("Development plugins missing, build with `just build-wasm`!"),
                    );

                    if id.contains("tc-tier3") {
                        config.version = UnresolvedVersionSpec::parse("1.2.3").ok();
                    }
                }
                other => {
                    return Err(ConfigError::Validator {
                        location: ".moon/toolchains.*".into(),
                        error: Box::new(ValidatorError {
                            errors: vec![ValidateError {
                                message:
                                    "a locator is required for plugins; accepts file paths and URLs"
                                        .into(),
                                path: Path::new(vec![
                                    PathSegment::Key(other.to_string()),
                                    PathSegment::Key("plugin".into()),
                                ]),
                            }],
                        }),
                        help: None,
                    }
                    .into());
                }
            };
        }

        Ok(())
    }
}
