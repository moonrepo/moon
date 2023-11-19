use proto_core::PluginLocator;
use schematic::Config;

/// Docs: https://moonrepo.dev/docs/config/toolchain#typescript
#[derive(Clone, Config, Debug)]
pub struct TypeScriptConfig {
    #[setting(default = true)]
    pub create_missing_config: bool,

    pub include_project_reference_sources: bool,

    pub include_shared_types: bool,

    // Not used but required by the toolchain macros!
    #[setting(skip)]
    pub plugin: Option<PluginLocator>,

    #[setting(default = "tsconfig.json")]
    pub project_config_file_name: String,

    #[setting(default = "tsconfig.json")]
    pub root_config_file_name: String,

    #[setting(default = "tsconfig.options.json")]
    pub root_options_config_file_name: String,

    pub route_out_dir_to_cache: bool,

    #[setting(default = true)]
    pub sync_project_references: bool,

    pub sync_project_references_to_paths: bool,

    #[setting(default = ".")]
    pub types_root: String,
}
