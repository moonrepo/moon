use moon_test_utils2::{create_empty_moon_sandbox, predicates::prelude::*};
use starbase_sandbox::assert_snapshot;
use std::fs;

mod hash {
    use super::*;

    #[test]
    fn errors_if_hash_doesnt_exist() {
        let sandbox = create_empty_moon_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg("a");
        });

        let output = assert.output();

        assert!(predicate::str::contains("Unable to find a hash manifest for a!").eval(&output));
    }

    #[test]
    fn prints_the_manifest() {
        let sandbox = create_empty_moon_sandbox();

        fs::create_dir_all(sandbox.path().join(".moon/cache/hashes")).unwrap();

        fs::write(
            sandbox.path().join(".moon/cache/hashes/a.json"),
            r#"{
    "command": "base",
    "args": [
        "a",
        "b",
        "c"
    ]
}"#,
        )
        .unwrap();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg("a");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn prints_the_manifest_in_json() {
        let sandbox = create_empty_moon_sandbox();

        fs::create_dir_all(sandbox.path().join(".moon/cache/hashes")).unwrap();

        fs::write(
            sandbox.path().join(".moon/cache/hashes/a.json"),
            r#"{
    "command": "base",
    "args": [
        "a",
        "b",
        "c"
    ]
}"#,
        )
        .unwrap();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg("a").arg("--json");
        });

        assert_snapshot!(assert.output());
    }
}

mod hash_diff {
    use super::*;

    #[test]
    fn errors_if_left_doesnt_exist() {
        let sandbox = create_empty_moon_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg("a").arg("b");
        });

        let output = assert.output();

        assert!(predicate::str::contains("Unable to find a hash manifest for a!").eval(&output));
    }

    #[test]
    fn errors_if_right_doesnt_exist() {
        let sandbox = create_empty_moon_sandbox();

        fs::create_dir_all(sandbox.path().join(".moon/cache/hashes")).unwrap();

        fs::write(
            sandbox.path().join(".moon/cache/hashes/a.json"),
            r#"{
    "command": "test",
    "args": [
        "a",
        "b",
        "c"
    ]
}"#,
        )
        .unwrap();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg("a").arg("b");
        });

        let output = assert.output();

        assert!(predicate::str::contains("Unable to find a hash manifest for b!").eval(&output));
    }

    #[test]
    fn prints_a_diff() {
        let sandbox = create_empty_moon_sandbox();

        fs::create_dir_all(sandbox.path().join(".moon/cache/hashes")).unwrap();

        fs::write(
            sandbox.path().join(".moon/cache/hashes/a.json"),
            r#"{
    "command": "base",
    "args": [
        "a",
        "b",
        "c"
    ]
}"#,
        )
        .unwrap();

        fs::write(
            sandbox.path().join(".moon/cache/hashes/b.json"),
            r#"{
    "command": "other",
    "args": [
        "a",
        "123",
        "c"
    ]
}"#,
        )
        .unwrap();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg("a").arg("b");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn prints_a_diff_in_json() {
        let sandbox = create_empty_moon_sandbox();

        fs::create_dir_all(sandbox.path().join(".moon/cache/hashes")).unwrap();

        fs::write(
            sandbox.path().join(".moon/cache/hashes/a.json"),
            r#"{
    "command": "base",
    "args": [
        "a",
        "b",
        "c"
    ]
}"#,
        )
        .unwrap();

        fs::write(
            sandbox.path().join(".moon/cache/hashes/b.json"),
            r#"{
    "command": "other",
    "args": [
        "a",
        "123",
        "c"
    ]
}"#,
        )
        .unwrap();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg("a").arg("b").arg("--json");
        });

        assert_snapshot!(assert.output());
    }
}
