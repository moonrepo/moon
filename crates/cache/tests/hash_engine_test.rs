use moon_cache::*;
use moon_hash::*;
use starbase_sandbox::create_empty_sandbox;
use std::fs;

hash_content!(
    struct Content<'l> {
        pub one: &'l str,
        pub two: usize,
    }
);

#[test]
fn saves_manifest() {
    let sandbox = create_empty_sandbox();
    let engine = HashEngine::new(sandbox.path()).unwrap();

    let mut hasher = ContentHasher::new("test");
    hasher
        .hash_content(Content {
            one: "abc",
            two: 123,
        })
        .unwrap();

    let hash = engine.save_manifest(&mut hasher).unwrap();

    assert_eq!(
        hash,
        "d612ce4d246bc531a35e693615e8cd2ca76f47b27a0a1ac768679154e0ba55c3"
    );

    let hash_path = sandbox
        .path()
        .join("hashes")
        .join("d612ce4d246bc531a35e693615e8cd2ca76f47b27a0a1ac768679154e0ba55c3.json");

    assert!(hash_path.exists());

    assert_eq!(
        fs::read_to_string(hash_path).unwrap(),
        r#"[{"one":"abc","two":123}]"#
    )
}

#[test]
fn saves_manifest_without_hasher() {
    let sandbox = create_empty_sandbox();
    let engine = HashEngine::new(sandbox.path()).unwrap();

    let hash = engine
        .save_manifest_without_hasher(
            "test",
            Content {
                one: "abc",
                two: 123,
            },
        )
        .unwrap();

    assert_eq!(
        hash,
        "d612ce4d246bc531a35e693615e8cd2ca76f47b27a0a1ac768679154e0ba55c3"
    );

    let hash_path = sandbox
        .path()
        .join("hashes")
        .join("d612ce4d246bc531a35e693615e8cd2ca76f47b27a0a1ac768679154e0ba55c3.json");

    assert!(hash_path.exists());

    assert_eq!(
        fs::read_to_string(hash_path).unwrap(),
        r#"[{"one":"abc","two":123}]"#
    )
}
