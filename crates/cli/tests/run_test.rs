use insta::assert_snapshot;
use moon_utils::path::replace_home_dir;
use moon_utils::test::{
    create_fixtures_sandbox, create_moon_command, create_moon_command_in, get_assert_output,
    get_fixtures_dir, replace_fixtures_dir,
};
use predicates::prelude::*;
use std::fs;

fn prepare_workspace(fixtures: &str) -> String {
    let root = get_fixtures_dir(fixtures);

    fs::read_to_string(root.join("package.json")).unwrap()
}

fn cleanup_workspace(fixtures: &str, package_json: String) {
    let root = get_fixtures_dir(fixtures);

    fs::write(root.join("package.json"), package_json).unwrap();

    let moon_cache = root.join(".moon/cache");

    if moon_cache.exists() {
        fs::remove_dir_all(moon_cache).unwrap();
    }

    // let node_modules = root.join("node_modules");

    // if node_modules.exists() {
    //     fs::remove_dir_all(node_modules).unwrap();
    // }

    let package_lock = root.join("package-lock.json");

    if package_lock.exists() {
        fs::remove_file(package_lock).unwrap();
    }

    let pnpm_lock = root.join("pnpm-lock.yaml");

    if pnpm_lock.exists() {
        fs::remove_file(pnpm_lock).unwrap();
    }

    let yarn_lock = root.join("yarn.lock");

    if yarn_lock.exists() {
        fs::remove_file(yarn_lock).unwrap();
    }
}

#[test]
fn errors_for_unknown_project() {
    let assert = create_moon_command("cases")
        .arg("run")
        .arg("unknown:test")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn errors_for_unknown_task_in_project() {
    let assert = create_moon_command("cases")
        .arg("run")
        .arg("base:unknown")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn errors_for_cycle_in_task_deps() {
    let assert = create_moon_command("cases")
        .arg("run")
        .arg("depsA:taskCycle")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

mod dependencies {
    use super::*;

    #[test]
    fn runs_the_graph_in_order() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("depsA:dependencyOrder")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn runs_the_graph_in_order_not_from_head() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("depsB:dependencyOrder")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}

mod target_scopes {
    use super::*;

    #[test]
    fn errors_for_deps_scope() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("^:test")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn errors_for_self_scope() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("~:test")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn supports_all_scope() {
        let assert = create_moon_command("cases").arg("run").arg(":all").assert();
        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("targetScopeA:all").eval(&output));
        assert!(predicate::str::contains("targetScopeB:all").eval(&output));
        assert!(predicate::str::contains("targetScopeC:all").eval(&output));
        assert!(predicate::str::contains("Tasks: 3 completed").eval(&output));
    }

    #[test]
    fn supports_deps_scope_in_task() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("targetScopeA:deps")
            .assert();
        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("targetScopeA:deps").eval(&output));
        assert!(predicate::str::contains("scope=deps").eval(&output));
        assert!(predicate::str::contains("depsA:standard").eval(&output));
        assert!(predicate::str::contains("deps=a").eval(&output));
        assert!(predicate::str::contains("depsB:standard").eval(&output));
        assert!(predicate::str::contains("deps=b").eval(&output));
        assert!(predicate::str::contains("depsC:standard").eval(&output));
        assert!(predicate::str::contains("deps=c").eval(&output));
        assert!(predicate::str::contains("Tasks: 4 completed").eval(&output));
    }

    #[test]
    fn supports_self_scope_in_task() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("targetScopeB:self")
            .assert();
        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("targetScopeB:self").eval(&output));
        assert!(predicate::str::contains("scope=self").eval(&output));
        assert!(predicate::str::contains("targetScopeB:selfOther").eval(&output));
        assert!(predicate::str::contains("selfOther").eval(&output));
        assert!(predicate::str::contains("Tasks: 2 completed").eval(&output));
    }
}

mod node {
    use super::*;

    fn get_path_safe_output(assert: &assert_cmd::assert::Assert, fixtures_dir: &str) -> String {
        replace_home_dir(&replace_fixtures_dir(
            &get_assert_output(assert),
            &get_fixtures_dir(fixtures_dir),
        ))
    }

    #[test]
    fn runs_package_managers() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:npm")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn runs_standard_script() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:standard")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn runs_cjs_files() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:cjs")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn runs_mjs_files() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:mjs")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn supports_top_level_await() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:topLevelAwait")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_zero() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:processExitZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_nonzero() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:processExitNonZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_code_zero() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:exitCodeZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_code_nonzero() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:exitCodeNonZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_throw_error() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:throwError")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, "cases"));
    }

    #[test]
    fn handles_unhandled_promise() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:unhandledPromise")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, "cases"));
    }

    #[test]
    fn passes_args_through() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:passthroughArgs")
            .arg("--")
            .arg("-aBc")
            .arg("--opt")
            .arg("value")
            .arg("--optCamel=value")
            .arg("foo")
            .arg("'bar baz'")
            .arg("--opt-kebab")
            .arg("123")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}

mod node_npm {
    use super::*;

    #[test]
    fn installs_correct_version() {
        let fixture = create_fixtures_sandbox("node-npm");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("npm:version")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn can_install_a_dep() {
        let fixture = create_fixtures_sandbox("node-npm");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("npm:installDep")
            .assert();

        assert.success();
    }
}

mod node_pnpm {
    use super::*;

    #[test]
    fn installs_correct_version() {
        let fixture = create_fixtures_sandbox("node-pnpm");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("pnpm:version")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn can_install_a_dep() {
        let fixture = create_fixtures_sandbox("node-pnpm");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("pnpm:installDep")
            .assert();

        assert.success();
    }
}

mod node_yarn1 {
    use super::*;

    #[test]
    fn installs_correct_version() {
        let fixture = create_fixtures_sandbox("node-yarn1");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("yarn:version")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn can_install_a_dep() {
        let fixture = create_fixtures_sandbox("node-yarn1");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("yarn:installDep")
            .assert();

        assert.success();
    }
}

mod system {
    use super::*;

    #[test]
    fn handles_echo() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("system:echo")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_ls() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("system:ls")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn runs_bash_script() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("system:bash")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn passes_args_through() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("system:passthroughArgs")
            .arg("--")
            .arg("-aBc")
            .arg("--opt")
            .arg("value")
            .arg("--optCamel=value")
            .arg("foo")
            .arg("'bar baz'")
            .arg("--opt-kebab")
            .arg("123")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}
