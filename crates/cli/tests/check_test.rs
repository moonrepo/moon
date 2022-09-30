use moon_utils::test::{create_moon_command, create_sandbox_with_git, get_assert_output};
use predicates::prelude::*;

#[test]
fn runs_tasks_in_project() {
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
        .arg("check")
        .arg("base")
        .assert();

    let output = get_assert_output(&assert);

    assert!(predicate::str::contains("base:base").eval(&output));
    assert!(predicate::str::contains("base:runFromProject").eval(&output));
    assert!(predicate::str::contains("base:runFromWorkspace").eval(&output));
    assert!(!predicate::str::contains("base:localOnly").eval(&output));
}

#[test]
fn runs_tasks_in_project_using_cwd() {
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path().join("base"))
        .arg("check")
        .assert();

    let output = get_assert_output(&assert);

    assert!(predicate::str::contains("base:base").eval(&output));
    assert!(predicate::str::contains("base:runFromProject").eval(&output));
    assert!(predicate::str::contains("base:runFromWorkspace").eval(&output));
    assert!(!predicate::str::contains("base:localOnly").eval(&output));
}

#[test]
fn runs_tasks_from_multiple_project() {
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
        .arg("check")
        .arg("base")
        .arg("noop")
        .assert();

    moon_utils::test::debug_sandbox(&fixture, &assert);

    let output = get_assert_output(&assert);

    assert!(predicate::str::contains("base:base").eval(&output));
    assert!(predicate::str::contains("base:runFromProject").eval(&output));
    assert!(predicate::str::contains("base:runFromWorkspace").eval(&output));
    assert!(!predicate::str::contains("base:localOnly").eval(&output));

    assert!(predicate::str::contains("noop:noop").eval(&output));
    assert!(predicate::str::contains("noop:noopWithDeps").eval(&output));
    assert!(predicate::str::contains("depsA:dependencyOrder").eval(&output)); // dep of noop
}
