use moon_hash::*;
use starbase_sandbox::create_empty_sandbox;
use std::fs;

content_hashable!(
    struct Content<'l> {
        pub one: &'l str,
        pub two: usize,
    }
);

impl<'l> ContentHashable for Content<'l> {}

#[test]
fn saves_manifest() {
    let sandbox = create_empty_sandbox();
    let engine = HashEngine::new(sandbox.path());

    let mut hasher = ContentHasher::new("test");
    hasher.hash(Content {
        one: "abc",
        two: 123,
    });

    let hash = engine.save_manifest(hasher).unwrap();

    assert_eq!(
        hash,
        "e5bfc3a1797a9546b04ed7a7d4ddf8e633381e5459640cca7a443bdef5b027ac"
    );

    let hash_path = sandbox
        .path()
        .join("hashes")
        .join("e5bfc3a1797a9546b04ed7a7d4ddf8e633381e5459640cca7a443bdef5b027ac.json");

    assert!(hash_path.exists());

    assert_eq!(
        fs::read_to_string(hash_path).unwrap(),
        r#"[
  {
    "one": "abc",
    "two": 123
  }
]"#
    )
}
