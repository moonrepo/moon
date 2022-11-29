use moon_test_utils::{
    create_sandbox_with_config, get_assert_output, get_cases_fixture_configs,
    predicates::prelude::*,
};

#[test]
fn runs_tasks_in_project() {
    let (workspace_config, toolchain_config, projects_config) = get_cases_fixture_configs();
    let sandbox = create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("check").arg("base");
    });

    let output = get_assert_output(&assert);

    assert!(predicate::str::contains("base:base").eval(&output));
    assert!(predicate::str::contains("base:runFromProject").eval(&output));
    assert!(predicate::str::contains("base:runFromWorkspace").eval(&output));
    assert!(!predicate::str::contains("base:localOnly").eval(&output));
}

#[test]
fn runs_tasks_in_project_using_cwd() {
    let (workspace_config, toolchain_config, projects_config) = get_cases_fixture_configs();
    let sandbox = create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let cwd = sandbox.path().join("base");
    let assert = sandbox.run_moon(|cmd| {
        cmd.current_dir(cwd).arg("check");
    });

    let output = get_assert_output(&assert);

    assert!(predicate::str::contains("base:base").eval(&output));
    assert!(predicate::str::contains("base:runFromProject").eval(&output));
    assert!(predicate::str::contains("base:runFromWorkspace").eval(&output));
    assert!(!predicate::str::contains("base:localOnly").eval(&output));
}

#[test]
fn runs_tasks_from_multiple_project() {
    let (workspace_config, toolchain_config, projects_config) = get_cases_fixture_configs();
    let sandbox = create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("check").arg("base").arg("noop");
    });

    let output = get_assert_output(&assert);

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
    let (workspace_config, toolchain_config, projects_config) = get_cases_fixture_configs();
    let sandbox = create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let cwd = sandbox.path().join("base");
    let assert = sandbox.run_moon(|cmd| {
        cmd.current_dir(cwd).arg("check").arg("--all");
    });

    assert.stderr(predicate::str::contains("all projects"));
}

#[test]
fn runs_on_all_projects_from_root_directory() {
    let (workspace_config, toolchain_config, projects_config) = get_cases_fixture_configs();
    let sandbox = create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("check").arg("--all");
    });

    assert.stderr(predicate::str::contains("all projects"));
}

mod reports {
    use super::*;

    #[test]
    fn does_not_create_a_report_by_default() {
        let (workspace_config, toolchain_config, projects_config) = get_cases_fixture_configs();
        let sandbox = create_sandbox_with_config(
            "cases",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("check").arg("base");
        });

        assert!(!sandbox.path().join(".moon/cache/runReport.json").exists());
    }

    #[test]
    fn creates_report_when_option_passed() {
        let (workspace_config, toolchain_config, projects_config) = get_cases_fixture_configs();
        let sandbox = create_sandbox_with_config(
            "cases",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("check").arg("base").arg("--report");
        });

        assert!(sandbox.path().join(".moon/cache/runReport.json").exists());
    }
}
