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

    let output = get_assert_output(&assert);

    assert!(predicate::str::contains("base:base").eval(&output));
    assert!(predicate::str::contains("base:runFromProject").eval(&output));
    assert!(predicate::str::contains("base:runFromWorkspace").eval(&output));
    assert!(!predicate::str::contains("base:localOnly").eval(&output));

    assert!(predicate::str::contains("noop:noop").eval(&output));
    assert!(predicate::str::contains("noop:noopWithDeps").eval(&output));
    assert!(predicate::str::contains("depsA:dependencyOrder").eval(&output)); // dep of noop
}

#[test]
fn runs_on_all_projects_from_root_directory() {
    let fixture = create_sandbox_with_git("cases");
    let assert = create_moon_command(fixture.path()).arg("check").assert();
    assert.stderr(predicate::str::contains("running check on all projects"));
}

mod reports {
    use super::*;

    #[test]
    fn doesnt_create_a_report_by_default() {
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("check")
            .arg("base")
            .assert();

        assert!(!fixture.path().join(".moon/cache/runReport.json").exists());
    }

    #[test]
    fn creates_report_when_option_passed() {
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("check")
            .arg("base")
            .arg("--report")
            .assert();

        assert!(fixture.path().join(".moon/cache/runReport.json").exists());
    }
}
