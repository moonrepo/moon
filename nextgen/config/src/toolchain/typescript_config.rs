use schematic::Config;

/// Docs: https://moonrepo.dev/docs/config/toolchain#typescript
#[derive(Config)]
pub struct TypeScriptConfig {
    #[setting(default = true)]
    pub create_missing_config: bool,

    #[setting(default_str = "tsconfig.json")]
    pub project_config_file_name: String,

    #[setting(default_str = "tsconfig.json")]
    pub root_config_file_name: String,

    #[setting(default_str = "tsconfig.options.json")]
    pub root_options_config_file_name: String,

    pub route_out_dir_to_cache: bool,

    #[setting(default = true)]
    pub sync_project_references: bool,

    pub sync_project_references_to_paths: bool,
}
