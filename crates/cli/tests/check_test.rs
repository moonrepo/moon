use moon_test_utils::{
    create_sandbox_with_config, get_cases_fixture_configs, predicates::prelude::*, Sandbox,
};

fn cases_sandbox() -> Sandbox {
    let (workspace_config, toolchain_config, tasks_config) = get_cases_fixture_configs();

    create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&tasks_config),
    )
}

#[test]
fn forces_cache_to_write_only() {
    let sandbox = cases_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("check").arg("base").arg("--updateCache");
    });

    let output = assert.output();

    assert!(!predicate::str::contains("cached").eval(&output));
}

#[test]
fn runs_tasks_in_project() {
    let sandbox = cases_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("check").arg("base");
    });

    let output = assert.output();

    assert!(predicate::str::contains("base:base").eval(&output));
    assert!(predicate::str::contains("base:runFromProject").eval(&output));
    assert!(predicate::str::contains("base:runFromWorkspace").eval(&output));
    assert!(!predicate::str::contains("base:localOnly").eval(&output));
}

#[test]
fn runs_tasks_in_project_using_cwd() {
    let sandbox = cases_sandbox();

    let cwd = sandbox.path().join("base");
    let assert = sandbox.run_moon(|cmd| {
        cmd.current_dir(cwd).arg("check");
    });

    let output = assert.output();

    assert!(predicate::str::contains("base:base").eval(&output));
    assert!(predicate::str::contains("base:runFromProject").eval(&output));
    assert!(predicate::str::contains("base:runFromWorkspace").eval(&output));
    assert!(!predicate::str::contains("base:localOnly").eval(&output));
}

#[test]
fn runs_tasks_from_multiple_project() {
    let sandbox = cases_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("check").arg("base").arg("noop");
    });

    assert.debug();

    let output = assert.output();

    assert!(predicate::str::contains("base:base").eval(&output));
    assert!(predicate::str::contains("base:runFromProject").eval(&output));
    assert!(predicate::str::contains("base:runFromWorkspace").eval(&output));
    assert!(!predicate::str::contains("base:localOnly").eval(&output));

    assert!(predicate::str::contains("noop:noop").eval(&output));
    assert!(predicate::str::contains("noop:noopWithDeps").eval(&output));
    assert!(predicate::str::contains("outputs:generateFile").eval(&output)); // dep of noop
}

#[test]
fn runs_for_all_projects_even_when_not_in_root_dir() {
    let sandbox = cases_sandbox();

    let cwd = sandbox.path().join("base");

    let assert = sandbox.run_moon(|cmd| {
        cmd.current_dir(cwd).arg("check").arg("--all");
    });

    assert
        .inner
        .stderr(predicate::str::contains("all projects"));
}

#[test]
fn runs_on_all_projects_from_root_directory() {
    let sandbox = cases_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("check").arg("--all");
    });

    assert
        .inner
        .stderr(predicate::str::contains("all projects"));
}

#[test]
fn creates_run_report() {
    let sandbox = cases_sandbox();

    sandbox.run_moon(|cmd| {
        cmd.arg("check").arg("base");
    });

    assert!(sandbox.path().join(".moon/cache/runReport.json").exists());
}
