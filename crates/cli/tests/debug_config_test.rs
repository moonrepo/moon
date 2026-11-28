use moon_test_utils::create_empty_moon_sandbox;
use starbase_sandbox::predicates::prelude::*;

mod debug_config {
    use super::*;

    #[test]
    fn inherits_prototools_versions_from_a_subdirectory() {
        let sandbox = create_empty_moon_sandbox();
        sandbox.create_file(".prototools", "node = \"20.1.0\"");
        sandbox.create_file(".moon/toolchains.yml", "node: {}");
        sandbox.create_file("packages/app/src/index.js", "");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("debug")
                .arg("config")
                .arg("--log")
                .arg("debug")
                .current_dir(sandbox.path().join("packages/app/src"));
        });

        assert
            .success()
            .stderr(predicate::str::contains("Inheriting node version 20.1.0"));
    }

    #[test]
    fn ignores_prototools_in_a_subdirectory() {
        let sandbox = create_empty_moon_sandbox();
        sandbox.create_file(".prototools", "node = \"20.1.0\"");
        sandbox.create_file("packages/app/.prototools", "node = \"18.0.0\"");
        sandbox.create_file(".moon/toolchains.yml", "node: {}");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("debug")
                .arg("config")
                .arg("--log")
                .arg("debug")
                .current_dir(sandbox.path().join("packages/app"));
        });

        assert
            .success()
            .stderr(predicate::str::contains("Inheriting node version 20.1.0"));
    }

    #[test]
    fn roots_the_proto_environment_at_the_workspace() {
        let sandbox = create_empty_moon_sandbox();
        sandbox.create_file("packages/app/src/index.js", "");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("debug")
                .arg("config")
                .current_dir(sandbox.path().join("packages/app/src"));
        });

        // proto mounts its working dir into plugins as `/cwd`, which would hide the
        // workspace above it from a plugin's upward search for the lockfile.
        let stderr = assert.stderr();
        let proto_env = stderr
            .split("&session.proto_env =")
            .nth(1)
            .and_then(|rest| rest.split("session.proto_env.load_config()").next())
            .unwrap();

        // Compare the folder name only: macOS resolves `/var` to `/private/var`.
        let name = sandbox.path().file_name().unwrap().to_str().unwrap();
        let working_dir = proto_env
            .lines()
            .find(|line| line.trim_start().starts_with("working_dir:"))
            .unwrap();

        assert!(working_dir.ends_with(&format!("{name}\",")), "{proto_env}");

        assert.success();
    }
}
