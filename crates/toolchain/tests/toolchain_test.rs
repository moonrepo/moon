use moon_config::WorkspaceConfig;
use moon_toolchain::Toolchain;
use predicates::prelude::*;
use std::env;
use std::path::Path;

pub fn create_toolchain(base_dir: &Path) -> Toolchain {
    let mut config = WorkspaceConfig::default();

    if let Some(ref mut node) = config.node {
        node.version = String::from("1.0.0");
    }

    Toolchain::from(&config, base_dir, &env::temp_dir()).unwrap()
}

#[test]
fn generates_paths() {
    let base_dir = assert_fs::TempDir::new().unwrap();
    let toolchain = create_toolchain(&base_dir);

    assert!(predicates::str::ends_with(".moon").eval(toolchain.dir.to_str().unwrap()));
    assert!(predicates::str::ends_with(".moon/temp").eval(toolchain.temp_dir.to_str().unwrap()));
    assert!(predicates::str::ends_with(".moon/tools").eval(toolchain.tools_dir.to_str().unwrap()));

    base_dir.close().unwrap();
}

#[test]
fn creates_dirs() {
    let base_dir = assert_fs::TempDir::new().unwrap();
    let home_dir = base_dir.join(".moon");
    let temp_dir = base_dir.join(".moon/temp");
    let tools_dir = base_dir.join(".moon/tools");

    assert!(!home_dir.exists());
    assert!(!temp_dir.exists());
    assert!(!tools_dir.exists());

    create_toolchain(&base_dir);

    assert!(home_dir.exists());
    assert!(temp_dir.exists());
    assert!(tools_dir.exists());

    base_dir.close().unwrap();
}

// #[test]
// fn loads_node_npm() {
//     let base_dir = assert_fs::TempDir::new().unwrap();
//     let toolchain = create_toolchain(&base_dir);

//     assert_ne!(toolchain.get_node(), None);
//     assert_ne!(toolchain.get_npm(), None);
//     assert_ne!(toolchain.get_npx(), None);
//     assert_eq!(toolchain.get_pnpm(), None);
//     assert_eq!(toolchain.get_yarn(), None);
//

//      base_dir.close().unwrap();
// }
