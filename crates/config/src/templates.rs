pub fn load_workspace_config_template() -> &'static str {
    include_str!("../templates/workspace.yml")
}

pub fn load_toolchain_config_template() -> &'static str {
    include_str!("../templates/toolchain.yml")
}

pub fn load_toolchain_bun_config_template() -> &'static str {
    include_str!("../templates/toolchain_bun.yml")
}

pub fn load_toolchain_deno_config_template() -> &'static str {
    include_str!("../templates/toolchain_deno.yml")
}

pub fn load_toolchain_node_config_template() -> &'static str {
    include_str!("../templates/toolchain_node.yml")
}

pub fn load_toolchain_rust_config_template() -> &'static str {
    include_str!("../templates/toolchain_rust.yml")
}

pub fn load_template_config_template() -> &'static str {
    include_str!("../templates/template.yml")
}
