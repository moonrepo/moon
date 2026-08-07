use moon_test_utils::create_empty_moon_sandbox;

#[test]
fn does_not_create_cache_directory() {
    let sandbox = create_empty_moon_sandbox();
    let cache_tag = sandbox.path().join(".moon/cache/CACHEDIR.TAG");

    assert!(!cache_tag.exists());

    sandbox
        .run_bin(|cmd| {
            cmd.arg("completions").arg("--shell").arg("bash");
        })
        .success();

    assert!(
        !cache_tag.exists(),
        "moon completions must not create .moon/cache/CACHEDIR.TAG"
    );
}

#[test]
fn does_not_create_cache_directory_for_nushell() {
    let sandbox = create_empty_moon_sandbox();
    let cache_tag = sandbox.path().join(".moon/cache/CACHEDIR.TAG");

    assert!(!cache_tag.exists());

    sandbox
        .run_bin(|cmd| {
            cmd.arg("completions").arg("--shell").arg("nu");
        })
        .success();

    assert!(
        !cache_tag.exists(),
        "moon completions nu must not create .moon/cache/CACHEDIR.TAG"
    );
}
