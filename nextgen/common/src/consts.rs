use std::path::{Path, PathBuf};

#[cfg(windows)]
pub const BIN_NAME: &str = "moon.exe";

#[cfg(not(windows))]
pub const BIN_NAME: &str = "moon";

pub const CONFIG_DIRNAME: &str = ".moon";

pub const CONFIG_TOOLCHAIN_FILENAME: &str = "toolchain.yml";

pub const CONFIG_WORKSPACE_FILENAME: &str = "workspace.yml";

pub const CONFIG_TASKS_FILENAME: &str = "tasks.yml";

pub const CONFIG_PROJECT_FILENAME: &str = "moon.yml";

pub const CONFIG_TEMPLATE_FILENAME: &str = "template.yml";

pub const PROTO_CLI_VERSION: &str = "0.31.4";

pub fn with_yaml_ext(name: &str) -> String {
    name.replace(".yml", ".yaml")
}

pub fn find_config_path(path: impl AsRef<Path>, name: &str) -> Option<PathBuf> {
    let path = path.as_ref();
    let yml_file = path.join(name);

    if yml_file.exists() {
        return Some(yml_file);
    }

    let yaml_file = path.join(with_yaml_ext(name));

    if yaml_file.exists() {
        return Some(yaml_file);
    }

    None
}
