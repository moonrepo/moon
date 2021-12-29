use moon_config::workspace::{PackageManager, YarnConfig};
use moon_config::WorkspaceConfig;
use moon_toolchain::tools::yarn::YarnTool;
use moon_toolchain::{Tool, Toolchain};
use predicates::prelude::*;
use std::env;

pub fn create_yarn_tool() -> (YarnTool, assert_fs::TempDir) {
    let base_dir = assert_fs::TempDir::new().unwrap();

    let mut config = WorkspaceConfig::default();

    config.node.version = String::from("1.0.0");
    config.node.package_manager = Some(PackageManager::Yarn);
    config.node.yarn = Some(YarnConfig {
        version: String::from("6.0.0"),
    });

    let toolchain = Toolchain::from(&config, base_dir.path(), &env::temp_dir()).unwrap();

    (toolchain.get_yarn().unwrap().to_owned(), base_dir)
}

#[test]
fn generates_paths() {
    let (yarn, temp_dir) = create_yarn_tool();

    assert!(predicates::str::ends_with(".moon/tools/node/1.0.0")
        .eval(yarn.get_install_dir().to_str().unwrap()));

    assert!(
        predicates::str::ends_with(".moon/tools/node/1.0.0/bin/yarn")
            .eval(yarn.get_bin_path().to_str().unwrap())
    );

    temp_dir.close().unwrap();
}

mod install {
    // TODO, how to test subprocesses?
}
