use monolith_config::WorkspaceConfig;
use monolith_toolchain::tools::npx::NpxTool;
use monolith_toolchain::{Tool, Toolchain};
use predicates::prelude::*;
use std::env;

pub fn create_npx_tool() -> (NpxTool, assert_fs::TempDir) {
    let base_dir = assert_fs::TempDir::new().unwrap();

    let mut config = WorkspaceConfig::default();

    config.node.version = String::from("1.0.0");

    let toolchain = Toolchain::from(&config, base_dir.path(), &env::temp_dir()).unwrap();

    (toolchain.get_npx().to_owned(), base_dir)
}

#[test]
fn generates_paths() {
    let (npx, temp_dir) = create_npx_tool();

    assert!(predicates::str::ends_with(".monolith/tools/node/1.0.0")
        .eval(npx.get_install_dir().to_str().unwrap()));

    assert!(
        predicates::str::ends_with(".monolith/tools/node/1.0.0/bin/npx")
            .eval(npx.get_bin_path().to_str().unwrap())
    );

    temp_dir.close().unwrap();
}

mod exec {
    // TODO, how to test subprocesses?
}
