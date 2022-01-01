use moon_config::workspace::{PackageManager, PnpmConfig};
use moon_config::WorkspaceConfig;
use moon_toolchain::tools::pnpm::PnpmTool;
use moon_toolchain::{Tool, Toolchain};
use predicates::prelude::*;
use std::env;

pub fn create_pnpm_tool() -> (PnpmTool, assert_fs::TempDir) {
    let base_dir = assert_fs::TempDir::new().unwrap();

    let mut config = WorkspaceConfig::default();

    if let Some(ref mut node) = config.node {
        node.version = String::from("1.0.0");
        node.package_manager = Some(PackageManager::Pnpm);
        node.pnpm = Some(PnpmConfig {
            version: String::from("6.0.0"),
        });
    }

    let toolchain = Toolchain::from(&config, base_dir.path(), &env::temp_dir()).unwrap();

    (toolchain.get_pnpm().unwrap().to_owned(), base_dir)
}

#[test]
fn generates_paths() {
    let (pnpm, temp_dir) = create_pnpm_tool();

    assert!(predicates::str::ends_with(".moon/tools/node/1.0.0")
        .eval(pnpm.get_install_dir().to_str().unwrap()));

    assert!(
        predicates::str::ends_with(".moon/tools/node/1.0.0/bin/pnpm")
            .eval(pnpm.get_bin_path().to_str().unwrap())
    );

    temp_dir.close().unwrap();
}

mod install {
    // TODO, how to test subprocesses?
}
