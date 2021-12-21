mod helpers;

use crate::helpers::{create_test_command, snap};

#[test]
fn no_projects() {
    let assert = create_test_command("base").arg("project-graph").assert();

    snap(&assert);
}

#[test]
fn many_projects() {
    let assert = create_test_command("projects")
        .arg("project-graph")
        .assert();

    snap(&assert);
}
