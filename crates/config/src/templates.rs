pub fn load_workspace_config_template() -> &'static str {
    include_str!("../templates/workspace.yml")
}

pub fn load_toolchains_config_template() -> &'static str {
    include_str!("../templates/toolchains.yml")
}

pub fn load_template_config_template() -> &'static str {
    include_str!("../templates/template.yml")
}
