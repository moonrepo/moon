use moon_bun_lang::load_lockfile_dependencies;
use starbase_sandbox::locate_fixture;
use std::sync::Arc;

#[test]
fn parses_lockfile() {
    let path = locate_fixture("bun.lock");
    let contents = std::fs::read_to_string(&path).unwrap();

    let _ = load_lockfile_dependencies(Arc::new(contents), path).unwrap();
}
