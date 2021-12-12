use dirs::home_dir;
use monolith_config::WorkspaceConfig;
use monolith_toolchain::Toolchain;
use std::env;

fn create_toolchain() -> Toolchain {
    let mut config = WorkspaceConfig::default();

    config.node.version = String::from("1.0.0");

    Toolchain::new(&config, &env::temp_dir()).unwrap()
}

#[test]
fn correct_paths() {
    let toolchain = create_toolchain();

    let mut home = home_dir().unwrap();
    home.push(".monolith");

    let mut temp = home.clone();
    temp.push("temp");

    let mut tools = home.clone();
    tools.push("tools");

    assert_eq!(toolchain.home_dir, home);
    assert_eq!(toolchain.temp_dir, temp);
    assert_eq!(toolchain.tools_dir, tools);
}
