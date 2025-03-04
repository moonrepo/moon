use moon_hash::{ContentHasher, hash_content};

hash_content!(
    struct ContentOne<'l> {
        pub one: &'l str,
    }
);

hash_content!(
    struct ContentTwo {
        pub two: usize,
    }
);

#[test]
fn hashes_empty() {
    let mut hasher = ContentHasher::new("test");

    assert_eq!(
        hasher.generate_hash().unwrap(),
        "4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945"
    );
}

#[test]
fn hashes_with_1_content() {
    let mut hasher = ContentHasher::new("test");
    hasher.hash_content(ContentOne { one: "abc" }).unwrap();

    assert_eq!(
        hasher.generate_hash().unwrap(),
        "37d04b9909c26008c08eeed62baf021fbd439a748c8a4b0aa27e66fe17c4dcb8"
    );

    let mut hasher = ContentHasher::new("test");
    hasher.hash_content(ContentOne { one: "xyz" }).unwrap();

    assert_eq!(
        hasher.generate_hash().unwrap(),
        "deec63985262a5c34ea2352e368aa96623193584ec1055817dcaaea1eb746c30"
    );
}

#[test]
fn hashes_with_2_content() {
    let mut hasher = ContentHasher::new("test");
    hasher.hash_content(ContentOne { one: "abc" }).unwrap();
    hasher.hash_content(ContentTwo { two: 123 }).unwrap();

    assert_eq!(
        hasher.generate_hash().unwrap(),
        "c65c4706a49bfa57a44b25bf5b441ec6549358c1a87a91a2aa8502fe225ac5f6"
    );

    let mut hasher = ContentHasher::new("test");
    hasher.hash_content(ContentOne { one: "xyz" }).unwrap();
    hasher.hash_content(ContentTwo { two: 789 }).unwrap();

    assert_eq!(
        hasher.generate_hash().unwrap(),
        "295892f785d11426dec31aa569e913db83d7a1cf2e944f82e55f1fdc33eccf96"
    );
}

#[test]
fn serializes_with_1_content() {
    let mut hasher = ContentHasher::new("test");
    hasher.hash_content(ContentOne { one: "abc" }).unwrap();

    assert_eq!(hasher.serialize().unwrap(), r#"[{"one":"abc"}]"#);

    let mut hasher = ContentHasher::new("test");
    hasher.hash_content(ContentOne { one: "xyz" }).unwrap();

    assert_eq!(hasher.serialize().unwrap(), r#"[{"one":"xyz"}]"#);
}

#[test]
fn serializes_with_2_content() {
    let mut hasher = ContentHasher::new("test");
    hasher.hash_content(ContentOne { one: "abc" }).unwrap();
    hasher.hash_content(ContentTwo { two: 123 }).unwrap();

    assert_eq!(
        hasher.serialize().unwrap(),
        r#"[{"one":"abc"},{"two":123}]"#
    );

    let mut hasher = ContentHasher::new("test");
    hasher.hash_content(ContentOne { one: "xyz" }).unwrap();
    hasher.hash_content(ContentTwo { two: 789 }).unwrap();

    assert_eq!(
        hasher.serialize().unwrap(),
        r#"[{"one":"xyz"},{"two":789}]"#
    );
}
