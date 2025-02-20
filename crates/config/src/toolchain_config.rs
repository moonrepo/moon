use crate::language_platform::*;
use crate::toolchain::*;
use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::{validate, Config};
use version_spec::UnresolvedVersionSpec;

#[cfg(feature = "proto")]
use crate::{inherit_tool, inherit_tool_without_version, is_using_tool_version};

/// Configures all tools and platforms.
/// Docs: https://moonrepo.dev/docs/config/toolchain
#[derive(Clone, Config, Debug)]
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

    /// Configures and enables the Python platform.
    #[setting(nested)]
    pub python: Option<PythonConfig>,

    /// Configures and enables the Rust platform.
    #[setting(nested)]
    pub rust: Option<RustConfig>,

    /// Configures and enables the TypeScript platform.
    #[setting(nested)]
    pub typescript: Option<TypeScriptConfig>,

    /// All configured toolchains by unique ID.
    #[setting(flatten, nested)]
    pub toolchains: FxHashMap<Id, ToolchainPluginConfig>,
}

impl ToolchainConfig {
    pub fn get_enabled(&self) -> Vec<Id> {
        let mut tools = self.toolchains.keys().cloned().collect::<Vec<_>>();

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

    pub fn get_version_env_vars(&self) -> FxHashMap<String, String> {
        let mut env = FxHashMap::default();

        let mut inject = |key: &str, version: &UnresolvedVersionSpec| {
            env.entry(key.to_owned())
                .or_insert_with(|| version.to_string());
        };

        if let Some(bun_config) = &self.bun {
            if let Some(version) = &bun_config.version {
                inject("PROTO_BUN_VERSION", version);
            }
        }

        if let Some(deno_config) = &self.deno {
            if let Some(version) = &deno_config.version {
                inject("PROTO_DENO_VERSION", version);
            }
        }

        if let Some(node_config) = &self.node {
            if let Some(version) = &node_config.version {
                inject("PROTO_NODE_VERSION", version);
            }

            if let Some(version) = &node_config.npm.version {
                inject("PROTO_NPM_VERSION", version);
            }

            if let Some(pnpm_config) = &node_config.pnpm {
                if let Some(version) = &pnpm_config.version {
                    inject("PROTO_PNPM_VERSION", version);
                }
            }

            if let Some(yarn_config) = &node_config.yarn {
                if let Some(version) = &yarn_config.version {
                    inject("PROTO_YARN_VERSION", version);
                }
            }

            if let Some(bunpm_config) = &node_config.bun {
                if let Some(version) = &bunpm_config.version {
                    inject("PROTO_BUN_VERSION", version);
                }
            }
        }

        if let Some(python_config) = &self.python {
            if let Some(version) = &python_config.version {
                inject("PROTO_PYTHON_VERSION", version);
            }

            if let Some(uv_config) = &python_config.uv {
                if let Some(version) = &uv_config.version {
                    inject("PROTO_UV_VERSION", version);
                }
            }
        }

        // We don't include Rust since it's a special case!

        env
    }
}

#[cfg(feature = "proto")]
impl ToolchainConfig {
    inherit_tool!(BunConfig, bun, "bun", inherit_proto_bun);

    inherit_tool!(DenoConfig, deno, "deno", inherit_proto_deno);

    inherit_tool!(NodeConfig, node, "node", inherit_proto_node);

    inherit_tool!(PythonConfig, python, "python", inherit_proto_python);

    inherit_tool!(RustConfig, rust, "rust", inherit_proto_rust);

    inherit_tool_without_version!(
        TypeScriptConfig,
        typescript,
        "typescript",
        inherit_proto_typescript
    );

    pub fn should_install_proto(&self) -> bool {
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

        false
    }

    pub fn inherit_proto(&mut self, proto_config: &proto_core::ProtoConfig) -> miette::Result<()> {
        self.inherit_proto_bun(proto_config)?;
        self.inherit_proto_deno(proto_config)?;
        self.inherit_proto_node(proto_config)?;
        self.inherit_proto_python(proto_config)?;
        self.inherit_proto_rust(proto_config)?;
        self.inherit_proto_typescript(proto_config)?;

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

        Ok(())
    }
}
