use moon_rust_lang::cargo_lock::*;
use moon_test_utils::{assert_debug_snapshot, create_sandbox};

#[test]
fn resolves_lockfile_dep_checksums() {
    let sandbox = create_sandbox("rust/workspaces");
    let deps = load_lockfile_dependencies(sandbox.path().join("Cargo.lock")).unwrap();

    assert_debug_snapshot!(deps);
}

#[test]
fn resolves_lockfile_dep_checksums_for_nonworkspace() {
    let sandbox = create_sandbox("rust/project");
    let deps = load_lockfile_dependencies(sandbox.path().join("Cargo.lock")).unwrap();

    assert_debug_snapshot!(deps);
}
