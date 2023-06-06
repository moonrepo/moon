use moon_hash::{content_hashable, ContentHashable, ContentHasher};

content_hashable!(
    struct ContentOne<'l> {
        pub one: &'l str,
    }
);

impl<'l> ContentHashable for ContentOne<'l> {}

content_hashable!(
    struct ContentTwo {
        pub two: usize,
    }
);

impl ContentHashable for ContentTwo {}

#[test]
fn hashes_empty() {
    let mut hasher = ContentHasher::new("test");

    assert_eq!(
        hasher.generate().unwrap(),
        "4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945"
    );
}

#[test]
fn hashes_with_1_content() {
    let mut hasher = ContentHasher::new("test");
    hasher.hash(ContentOne { one: "abc" });

    assert_eq!(
        hasher.generate().unwrap(),
        "22ecfb9f32de525b6ab34a4c9e6b96dd9eee6c0873823b8dacfe586a1c4ec553"
    );

    let mut hasher = ContentHasher::new("test");
    hasher.hash(ContentOne { one: "xyz" });

    assert_eq!(
        hasher.generate().unwrap(),
        "91d2e04331bdbe1e4836e32f9f2be57c80442cacb03955414347f3c9a82d3930"
    );
}

#[test]
fn hashes_with_2_content() {
    let mut hasher = ContentHasher::new("test");
    hasher.hash(ContentOne { one: "abc" });
    hasher.hash(ContentTwo { two: 123 });

    assert_eq!(
        hasher.generate().unwrap(),
        "9184b4ed2b6ebf4fe1b843cdf8705e749929b7a6910d0d6a0325f4d06b435291"
    );

    let mut hasher = ContentHasher::new("test");
    hasher.hash(ContentOne { one: "xyz" });
    hasher.hash(ContentTwo { two: 789 });

    assert_eq!(
        hasher.generate().unwrap(),
        "efc26aa8f0742cbc84ee91c78a2a6ecf96b8331f85db733f3d3dbf0ab01d7c16"
    );
}

#[test]
fn serializes_with_1_content() {
    let mut hasher = ContentHasher::new("test");
    hasher.hash(ContentOne { one: "abc" });

    assert_eq!(
        hasher.serialize().unwrap(),
        r#"[
  {
    "one": "abc"
  }
]"#
    );

    let mut hasher = ContentHasher::new("test");
    hasher.hash(ContentOne { one: "xyz" });

    assert_eq!(
        hasher.serialize().unwrap(),
        r#"[
  {
    "one": "xyz"
  }
]"#
    );
}

#[test]
fn serializes_with_2_content() {
    let mut hasher = ContentHasher::new("test");
    hasher.hash(ContentOne { one: "abc" });
    hasher.hash(ContentTwo { two: 123 });

    assert_eq!(
        hasher.serialize().unwrap(),
        r#"[
  {
    "one": "abc"
  },
  {
    "two": 123
  }
]"#
    );

    let mut hasher = ContentHasher::new("test");
    hasher.hash(ContentOne { one: "xyz" });
    hasher.hash(ContentTwo { two: 789 });

    assert_eq!(
        hasher.serialize().unwrap(),
        r#"[
  {
    "one": "xyz"
  },
  {
    "two": 789
  }
]"#
    );
}
