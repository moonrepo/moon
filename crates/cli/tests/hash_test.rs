use moon_test_utils::{create_empty_moon_sandbox, predicates::prelude::*};
use starbase_sandbox::{Sandbox, assert_snapshot};
use std::fs;

/// Hash manifests live in the local CAS, which shards objects by the first 2
/// chars of their 64-char hash. Tests seed them directly so they can address a
/// manifest by an abbreviated hash the way a user does.
fn seed_manifest(sandbox: &Sandbox, hash: &str, contents: &str) {
    let dir = sandbox.path().join(".moon/cache/blobs").join(&hash[0..2]);

    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(&hash[2..]), contents).unwrap();
}

fn hash_a() -> String {
    "a".repeat(64)
}

fn hash_b() -> String {
    "b".repeat(64)
}

const BASE_MANIFEST: &str = r#"{
    "command": "base",
    "args": [
        "a",
        "b",
        "c"
    ]
}"#;

const OTHER_MANIFEST: &str = r#"{
    "command": "other",
    "args": [
        "a",
        "123",
        "c"
    ]
}"#;

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
    fn errors_if_the_partial_hash_is_ambiguous() {
        // Two manifests share the abbreviation, so moon can't know which was
        // meant and must say so instead of picking one.
        let sandbox = create_empty_moon_sandbox();

        seed_manifest(&sandbox, &format!("aa{}", "0".repeat(62)), BASE_MANIFEST);
        seed_manifest(&sandbox, &format!("aa{}", "1".repeat(62)), OTHER_MANIFEST);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg("aa");
        });

        let output = assert.output();

        assert!(
            predicate::str::contains("Found multiple hash manifests starting with aa")
                .eval(&output)
        );
    }

    #[test]
    fn prints_the_manifest() {
        let sandbox = create_empty_moon_sandbox();

        seed_manifest(&sandbox, &hash_a(), BASE_MANIFEST);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg("a");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn prints_the_manifest_from_a_full_hash() {
        let sandbox = create_empty_moon_sandbox();

        seed_manifest(&sandbox, &hash_a(), BASE_MANIFEST);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg(hash_a());
        });

        let output = assert.output();

        assert!(predicate::str::contains("\"command\": \"base\"").eval(&output));
    }

    #[test]
    fn prints_the_manifest_in_json() {
        let sandbox = create_empty_moon_sandbox();

        seed_manifest(&sandbox, &hash_a(), BASE_MANIFEST);

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

        seed_manifest(&sandbox, &hash_a(), BASE_MANIFEST);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg("a").arg("b");
        });

        let output = assert.output();

        assert!(predicate::str::contains("Unable to find a hash manifest for b!").eval(&output));
    }

    #[test]
    fn prints_a_diff() {
        let sandbox = create_empty_moon_sandbox();

        seed_manifest(&sandbox, &hash_a(), BASE_MANIFEST);
        seed_manifest(&sandbox, &hash_b(), OTHER_MANIFEST);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg("a").arg("b");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn prints_a_diff_in_json() {
        let sandbox = create_empty_moon_sandbox();

        seed_manifest(&sandbox, &hash_a(), BASE_MANIFEST);
        seed_manifest(&sandbox, &hash_b(), OTHER_MANIFEST);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg("a").arg("b").arg("--json");
        });

        assert_snapshot!(assert.output());
    }
}
