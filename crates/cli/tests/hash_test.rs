use moon_test_utils::{create_empty_moon_sandbox, predicates::prelude::*};
use starbase_sandbox::{Sandbox, assert_snapshot};
use std::fs;

/// Hash fingerprints live in the local objects store, which shards them by the
/// first 2 chars of their 64-char hash. Tests seed them directly so they can
/// address a fingerprint by an abbreviated hash the way a user does.
fn seed_fingerprint(sandbox: &Sandbox, hash: &str, contents: &str) {
    let dir = sandbox.path().join(".moon/cache/objects").join(&hash[0..2]);

    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(&hash[2..]), contents).unwrap();
}

fn hash_a() -> String {
    "a".repeat(64)
}

fn hash_b() -> String {
    "b".repeat(64)
}

const BASE_FINGERPRINT: &str = r#"{
    "command": "base",
    "args": [
        "a",
        "b",
        "c"
    ]
}"#;

const OTHER_FINGERPRINT: &str = r#"{
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

        assert!(predicate::str::contains("Unable to find a hash fingerprint for a!").eval(&output));
    }

    #[test]
    fn errors_if_the_partial_hash_is_ambiguous() {
        // Two fingerprints share the abbreviation, so moon can't know which was
        // meant and must say so instead of picking one.
        let sandbox = create_empty_moon_sandbox();

        seed_fingerprint(&sandbox, &format!("aa{}", "0".repeat(62)), BASE_FINGERPRINT);
        seed_fingerprint(
            &sandbox,
            &format!("aa{}", "1".repeat(62)),
            OTHER_FINGERPRINT,
        );

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg("aa");
        });

        let output = assert.output();

        assert!(
            predicate::str::contains("Found multiple hash fingerprints starting with aa")
                .eval(&output)
        );
    }

    #[test]
    fn prints_the_fingerprint() {
        let sandbox = create_empty_moon_sandbox();

        seed_fingerprint(&sandbox, &hash_a(), BASE_FINGERPRINT);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg("a");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn prints_the_fingerprint_from_a_full_hash() {
        let sandbox = create_empty_moon_sandbox();

        seed_fingerprint(&sandbox, &hash_a(), BASE_FINGERPRINT);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg(hash_a());
        });

        let output = assert.output();

        assert!(predicate::str::contains("\"command\": \"base\"").eval(&output));
    }

    #[test]
    fn prints_the_fingerprint_in_json() {
        let sandbox = create_empty_moon_sandbox();

        seed_fingerprint(&sandbox, &hash_a(), BASE_FINGERPRINT);

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

        assert!(predicate::str::contains("Unable to find a hash fingerprint for a!").eval(&output));
    }

    #[test]
    fn errors_if_right_doesnt_exist() {
        let sandbox = create_empty_moon_sandbox();

        seed_fingerprint(&sandbox, &hash_a(), BASE_FINGERPRINT);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg("a").arg("b");
        });

        let output = assert.output();

        assert!(predicate::str::contains("Unable to find a hash fingerprint for b!").eval(&output));
    }

    #[test]
    fn prints_a_diff() {
        let sandbox = create_empty_moon_sandbox();

        seed_fingerprint(&sandbox, &hash_a(), BASE_FINGERPRINT);
        seed_fingerprint(&sandbox, &hash_b(), OTHER_FINGERPRINT);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg("a").arg("b");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn prints_a_diff_in_json() {
        let sandbox = create_empty_moon_sandbox();

        seed_fingerprint(&sandbox, &hash_a(), BASE_FINGERPRINT);
        seed_fingerprint(&sandbox, &hash_b(), OTHER_FINGERPRINT);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("hash").arg("a").arg("b").arg("--json");
        });

        assert_snapshot!(assert.output());
    }
}
