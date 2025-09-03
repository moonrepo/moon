use crate::config_struct;
use crate::language_platform::*;
use crate::toolchain::*;
use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::{Config, validate};
use version_spec::UnresolvedVersionSpec;

#[cfg(feature = "proto")]
use crate::{inherit_tool, is_using_tool_version};

config_struct!(
    /// Configures all tools and platforms.
    /// Docs: https://moonrepo.dev/docs/config/toolchain
    #[derive(Config)]
    #[config(allow_unknown_fields)]
    pub struct ToolchainConfig {
        #[setting(
            default = "https://moonrepo.dev/schemas/toolchain.json",
            rename = "$schema"
        )]
        pub schema: String,

        /// Extends one or many toolchain configuration files.
        /// Supports a relative file path or a secure URL.
        #[setting(extend, validate = validate::extends_from)]
        pub extends: Option<schematic::ExtendsFrom>,

        /// Configures and enables the Bun platform.
        #[setting(nested)]
        pub bun: Option<BunConfig>,

        /// Configures and enables the Deno platform.
        #[setting(nested)]
        pub deno: Option<DenoConfig>,

        /// Configures moon itself.
        #[setting(nested)]
        pub moon: MoonConfig,

        /// Configures and enables the Node.js platform.
        #[setting(nested)]
        pub node: Option<NodeConfig>,

        /// Configures how moon integrates with proto.
        #[setting(nested)]
        pub proto: ProtoConfig,

        /// Configures and enables the Python platform.
        #[setting(nested)]
        pub python: Option<PythonConfig>,

        /// Configures and enables the Rust platform.
        #[setting(nested)]
        pub rust: Option<RustConfig>,

        /// All configured toolchains by unique ID.
        #[setting(flatten, nested)]
        pub plugins: FxHashMap<Id, ToolchainPluginConfig>,
    }
);

impl ToolchainConfig {
    pub fn get_enabled(&self) -> Vec<Id> {
        let mut tools = self.plugins.keys().cloned().collect::<Vec<_>>();

        if self.bun.is_some() {
            tools.push(Id::raw("bun"));
        }

        if self.deno.is_some() {
            tools.push(Id::raw("deno"));
        }

        if let Some(node) = &self.node {
            tools.push(Id::raw("node"));

            // Better way to handle this?
            if node.bun.is_some() || matches!(node.package_manager, NodePackageManager::Bun) {
                tools.push(Id::raw("bun"));
            }
        }

        if self.python.is_some() {
            tools.push(Id::raw("python"))
        }

        if self.rust.is_some() {
            tools.push(Id::raw("rust"));
        }

        tools.push(Id::raw("system"));
        tools
    }

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

        if self.python.is_some() {
            tools.push(PlatformType::Python)
        }

        if self.rust.is_some() {
            tools.push(PlatformType::Rust);
        }

        tools
    }

    pub fn get_plugin_config(&self, id: impl AsRef<str>) -> Option<&ToolchainPluginConfig> {
        let (stable_id, unstable_id) = Id::stable_and_unstable(id);

        self.plugins
            .get(&stable_id)
            .or_else(|| self.plugins.get(&unstable_id))
    }

    #[cfg(feature = "proto")]
    pub fn get_plugin_locator(id: &Id) -> Option<proto_core::PluginLocator> {
        use proto_core::warpgate::find_debug_locator_with_url_fallback;

        match id.as_str() {
            "typescript" => Some(find_debug_locator_with_url_fallback(
                "typescript_toolchain",
                "0.2.3",
            )),
            "unstable_bun" => Some(find_debug_locator_with_url_fallback(
                "bun_toolchain",
                "0.1.1",
            )),
            "unstable_javascript" => Some(find_debug_locator_with_url_fallback(
                "javascript_toolchain",
                "0.1.2",
            )),
            "unstable_go" => Some(find_debug_locator_with_url_fallback(
                "go_toolchain",
                "0.1.5",
            )),
            "unstable_node" => Some(find_debug_locator_with_url_fallback(
                "node_toolchain",
                "0.1.1",
            )),
            "unstable_npm" => Some(find_debug_locator_with_url_fallback(
                "node_depman_toolchain",
                "0.1.1",
            )),
            "unstable_pnpm" => Some(find_debug_locator_with_url_fallback(
                "node_depman_toolchain",
                "0.1.1",
            )),
            "unstable_rust" => Some(find_debug_locator_with_url_fallback(
                "rust_toolchain",
                "0.2.4",
            )),
            "unstable_yarn" => Some(find_debug_locator_with_url_fallback(
                "node_depman_toolchain",
                "0.1.1",
            )),
            _ => None,
        }
    }

    pub fn get_version_env_vars(&self) -> FxHashMap<String, String> {
        let mut env = FxHashMap::default();

        let mut inject = |key: &str, version: &UnresolvedVersionSpec| {
            env.entry(key.to_owned())
                .or_insert_with(|| version.to_string());
        };

        if let Some(bun_config) = &self.bun
            && let Some(version) = &bun_config.version
        {
            inject("PROTO_BUN_VERSION", version);
        }

        if let Some(deno_config) = &self.deno
            && let Some(version) = &deno_config.version
        {
            inject("PROTO_DENO_VERSION", version);
        }

        if let Some(node_config) = &self.node {
            if let Some(version) = &node_config.version {
                inject("PROTO_NODE_VERSION", version);
            }

            if let Some(version) = &node_config.npm.version {
                inject("PROTO_NPM_VERSION", version);
            }

            if let Some(pnpm_config) = &node_config.pnpm
                && let Some(version) = &pnpm_config.version
            {
                inject("PROTO_PNPM_VERSION", version);
            }

            if let Some(yarn_config) = &node_config.yarn
                && let Some(version) = &yarn_config.version
            {
                inject("PROTO_YARN_VERSION", version);
            }

            if let Some(bunpm_config) = &node_config.bun
                && let Some(version) = &bunpm_config.version
            {
                inject("PROTO_BUN_VERSION", version);
            }
        }

        if let Some(python_config) = &self.python {
            if let Some(version) = &python_config.version {
                inject("PROTO_PYTHON_VERSION", version);
            }

            if let Some(uv_config) = &python_config.uv
                && let Some(version) = &uv_config.version
            {
                inject("PROTO_UV_VERSION", version);
            }
        }

        // We don't include Rust since it's a special case!

        env
    }

    pub fn is_plugin(&self, id: &str) -> bool {
        self.plugins.contains_key(id)
    }
}

#[cfg(feature = "proto")]
impl ToolchainConfig {
    inherit_tool!(BunConfig, bun, "bun", inherit_proto_bun);

    inherit_tool!(DenoConfig, deno, "deno", inherit_proto_deno);

    inherit_tool!(NodeConfig, node, "node", inherit_proto_node);

    inherit_tool!(PythonConfig, python, "python", inherit_proto_python);

    inherit_tool!(RustConfig, rust, "rust", inherit_proto_rust);

    pub fn requires_proto(&self) -> bool {
        is_using_tool_version!(self, bun);
        is_using_tool_version!(self, deno);
        is_using_tool_version!(self, node);
        is_using_tool_version!(self, node, bun);
        is_using_tool_version!(self, node, pnpm);
        is_using_tool_version!(self, node, yarn);
        is_using_tool_version!(self, python);
        is_using_tool_version!(self, python, uv);
        is_using_tool_version!(self, rust);

        // Special case
        if self
            .node
            .as_ref()
            .is_some_and(|config| config.npm.version.is_some())
        {
            return true;
        }

        for config in self.plugins.values() {
            if config.version.is_some() {
                return true;
            }
        }

        false
    }

    pub fn inherit_proto_for_plugins(
        &mut self,
        proto_config: &proto_core::ProtoConfig,
    ) -> miette::Result<()> {
        use moon_common::color;
        use proto_core::ToolContext;
        use tracing::trace;

        for (id, config) in &mut self.plugins {
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

    pub fn inherit_proto(&mut self, proto_config: &proto_core::ProtoConfig) -> miette::Result<()> {
        use tracing::warn;

        self.inherit_proto_for_plugins(proto_config)?;
        self.inherit_proto_bun(proto_config)?;
        self.inherit_proto_deno(proto_config)?;
        self.inherit_proto_node(proto_config)?;
        self.inherit_proto_python(proto_config)?;
        self.inherit_proto_rust(proto_config)?;

        if let Some(node_config) = &mut self.node {
            node_config.inherit_proto(proto_config)?;

            // If bun and node are both enabled, and bun is being used
            // as a package manager within node, we need to keep the
            // versions in sync between both tools. The bun toolchain
            // version takes precedence!
            if let (Some(bun_config), Some(bunpm_config)) = (&mut self.bun, &mut node_config.bun) {
                if bun_config.version.is_some() && bunpm_config.version.is_none() {
                    bunpm_config.version = bun_config.version.clone();
                } else if bunpm_config.version.is_some() && bun_config.version.is_none() {
                    bun_config.version = bunpm_config.version.clone();
                }

                if !bun_config.install_args.is_empty() && bunpm_config.install_args.is_empty() {
                    bunpm_config.install_args = bun_config.install_args.clone();
                } else if !bunpm_config.install_args.is_empty()
                    && bun_config.install_args.is_empty()
                {
                    bun_config.install_args = bunpm_config.install_args.clone();
                }
            };
        }

        if let Some(python_config) = &mut self.python {
            python_config.inherit_proto(proto_config)?;
        }

        self.inherit_plugin_locators()?;

        if self.bun.is_some()
            && (self.plugins.contains_key("bun")
                || self.plugins.contains_key("unstable_bun")
                || self.plugins.contains_key("javascript")
                || self.plugins.contains_key("unstable_javascript"))
        {
            warn!(
                "The legacy Bun platform and WASM based JavaScript/Bun toolchains must not be used together!"
            );
        }

        if self.node.is_some()
            && (self.plugins.contains_key("node")
                || self.plugins.contains_key("unstable_node")
                || self.plugins.contains_key("javascript")
                || self.plugins.contains_key("unstable_javascript"))
        {
            warn!(
                "The legacy Node.js platform and WASM based JavaScript/Node.js toolchains must not be used together!"
            );
        }

        if self.rust.is_some()
            && (self.plugins.contains_key("rust") || self.plugins.contains_key("unstable_rust"))
        {
            warn!(
                "The legacy Rust platform and WASM based Rust toolchain must not be used together!"
            );
        }

        Ok(())
    }

    pub fn inherit_default_plugins(&mut self) -> miette::Result<()> {
        for id in [
            "typescript",
            "unstable_bun",
            "unstable_go",
            "unstable_javascript",
            "unstable_node",
            "unstable_npm",
            "unstable_rust",
            // We only need 1 package manager while testing!
            // "unstable_pnpm",
            // "unstable_yarn",
        ] {
            if !self.plugins.contains_key(id) {
                self.plugins
                    .insert(Id::raw(id), ToolchainPluginConfig::default());
            }
        }

        Ok(())
    }

    pub fn inherit_plugin_locators(&mut self) -> miette::Result<()> {
        use schematic::{ConfigError, Path, PathSegment, ValidateError, ValidatorError};

        for (id, config) in self.plugins.iter_mut() {
            if config.plugin.is_some() {
                continue;
            }

            match id.as_str() {
                "typescript"
                | "unstable_bun"
                | "unstable_go"
                | "unstable_javascript"
                | "unstable_node"
                | "unstable_npm"
                | "unstable_pnpm"
                | "unstable_rust"
                | "unstable_yarn" => {
                    config.plugin = Self::get_plugin_locator(id);
                }
                other => {
                    return Err(ConfigError::Validator {
                        location: ".moon/toolchain.yml".into(),
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
