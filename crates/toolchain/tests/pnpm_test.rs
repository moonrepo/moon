use monolith_config::workspace::{PackageManager, PnpmConfig};
use monolith_config::WorkspaceConfig;
use monolith_toolchain::tools::pnpm::PnpmTool;
use monolith_toolchain::{Tool, Toolchain};
use predicates::prelude::*;
use std::env;

pub fn create_pnpm_tool() -> (PnpmTool, assert_fs::TempDir) {
    let base_dir = assert_fs::TempDir::new().unwrap();

    let mut config = WorkspaceConfig::default();

    config.node.version = String::from("1.0.0");
    config.node.package_manager = Some(PackageManager::Pnpm);
    config.node.pnpm = Some(PnpmConfig {
        version: String::from("6.0.0"),
    });

    let toolchain = Toolchain::from(&config, base_dir.path(), &env::temp_dir()).unwrap();

    (toolchain.get_pnpm().unwrap().to_owned(), base_dir)
}

#[test]
fn generates_paths() {
    let (pnpm, temp_dir) = create_pnpm_tool();

    assert!(predicates::str::ends_with(".monolith/tools/node/1.0.0")
        .eval(pnpm.get_install_dir().to_str().unwrap()));

    assert!(
        predicates::str::ends_with(".monolith/tools/node/1.0.0/bin/pnpm")
            .eval(pnpm.get_bin_path().to_str().unwrap())
    );

    temp_dir.close().unwrap();
}

mod install {
    // TODO, how to test subprocesses?
}
