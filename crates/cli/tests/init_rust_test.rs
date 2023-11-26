use moon_test_utils::{create_sandbox, predicates::prelude::*};
use std::fs;

mod init_rust {
    use super::*;

    #[test]
    fn infers_version_from_toolchain_toml() {
        let sandbox = create_sandbox("init-sandbox");
        let root = sandbox.path().to_path_buf();
        let config = root.join(".moon").join("toolchain.yml");

        sandbox.create_file("Cargo.toml", "");
        sandbox.create_file("rust-toolchain.toml", "[toolchain]\nchannel = \"1.2.3\"");

        sandbox.run_moon(|cmd| {
            cmd.arg("init")
                .arg("rust")
                .arg("--yes")
                .arg("--to")
                .arg(root);
        });

        let content = fs::read_to_string(config).unwrap();

        assert!(predicate::str::contains("version: '1.2.3'").eval(&content));
    }
}
