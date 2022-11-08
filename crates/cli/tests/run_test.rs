mod utils;

use insta::assert_snapshot;
use moon_cache::CacheEngine;
use moon_utils::path::standardize_separators;
use moon_utils::test::{
    create_moon_command, create_sandbox, create_sandbox_with_git, get_assert_output,
};
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use utils::get_path_safe_output;

async fn extract_hash_from_run(fixture: &Path, target: &str) -> String {
    let engine = CacheEngine::load(fixture).await.unwrap();
    let cache = engine.cache_run_target_state(target).await.unwrap();

    cache.hash
}

#[test]
fn errors_for_unknown_project() {
    let fixture = create_sandbox("cases");

    let assert = create_moon_command(fixture.path())
        .arg("run")
        .arg("unknown:test")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn errors_for_unknown_task_in_project() {
    let fixture = create_sandbox("cases");

    let assert = create_moon_command(fixture.path())
        .arg("run")
        .arg("base:unknown")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn errors_for_unknown_all_target() {
    let fixture = create_sandbox("cases");

    let assert = create_moon_command(fixture.path())
        .arg("run")
        .arg(":unknown")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn errors_for_cycle_in_task_deps() {
    let fixture = create_sandbox("cases");

    let assert = create_moon_command(fixture.path())
        .arg("run")
        .arg("depsA:taskCycle")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[cfg(not(windows))]
mod general {
    use super::*;
    use utils::append_workspace_config;

    #[test]
    fn logs_command_for_project_root() {
        let fixture = create_sandbox_with_git("cases");

        append_workspace_config(fixture.path(), "runner:\n  logRunningCommand: true");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("base:runFromProject")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn logs_command_for_workspace_root() {
        let fixture = create_sandbox_with_git("cases");

        append_workspace_config(fixture.path(), "runner:\n  logRunningCommand: true");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("base:runFromWorkspace")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}

mod configs {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn bubbles_up_invalid_workspace_config() {
        let fixture = create_sandbox("config-invalid-workspace");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("project:task")
            .assert();

        assert_snapshot!(standardize_separators(get_path_safe_output(
            &assert,
            &PathBuf::from("./fake/path")
        )));
    }

    #[test]
    fn bubbles_up_invalid_global_project_config() {
        let fixture = create_sandbox("config-invalid-global-project");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("project:task")
            .assert();

        assert_snapshot!(standardize_separators(get_path_safe_output(
            &assert,
            &PathBuf::from("./fake/path")
        )));
    }

    #[test]
    fn bubbles_up_invalid_project_config() {
        let fixture = create_sandbox("config-invalid-project");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("test:task")
            .assert();

        assert_snapshot!(standardize_separators(get_path_safe_output(
            &assert,
            &PathBuf::from("./fake/path")
        )));
    }
}

mod logs {
    use super::*;
    use moon_utils::test::create_sandbox_with_git;

    #[test]
    fn creates_log_file() {
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("--logFile=output.log")
            .arg("run")
            .arg("node:standard")
            .assert();

        let output_path = fixture.path().join("output.log");

        assert!(output_path.exists());
    }

    #[test]
    fn creates_nested_log_file() {
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("--logFile=nested/output.log")
            .arg("run")
            .arg("node:standard")
            .assert();

        let output_path = fixture.path().join("nested/output.log");

        assert!(output_path.exists());
    }
}

mod dependencies {
    use super::*;

    #[test]
    fn runs_the_graph_in_order() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("depsA:dependencyOrder")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn runs_the_graph_in_order_not_from_head() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("depsB:dependencyOrder")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn can_run_deps_in_serial() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("dependsOn:serialDeps")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[tokio::test]
    async fn generates_unique_hashes_for_each_target() {
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("run")
            .arg("outputs:withDeps")
            .assert();

        assert_eq!(
            extract_hash_from_run(fixture.path(), "outputs:asDep").await,
            "eb42c930249c065743d1ad796c966c92c87f1a091e0a517fea957b983332ad4e"
        );
        assert_eq!(
            extract_hash_from_run(fixture.path(), "outputs:withDeps").await,
            "8ee5353d64ec1a5cf7b6dc9d453668ee5b504da90a0bdd975015a10b8a6577d4"
        );
    }

    #[tokio::test]
    async fn changes_primary_hash_if_deps_hash_changes() {
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("run")
            .arg("outputs:withDeps")
            .assert();

        assert_eq!(
            extract_hash_from_run(fixture.path(), "outputs:asDep").await,
            "eb42c930249c065743d1ad796c966c92c87f1a091e0a517fea957b983332ad4e"
        );
        assert_eq!(
            extract_hash_from_run(fixture.path(), "outputs:withDeps").await,
            "8ee5353d64ec1a5cf7b6dc9d453668ee5b504da90a0bdd975015a10b8a6577d4"
        );

        // Create an `inputs` file for `outputs:asDep`
        fs::write(fixture.path().join("outputs/random.js"), "").unwrap();

        create_moon_command(fixture.path())
            .arg("run")
            .arg("outputs:withDeps")
            .assert();

        assert_eq!(
            extract_hash_from_run(fixture.path(), "outputs:asDep").await,
            "56d28527480e18d56eaafb3316a374103db405b4ce91aee37b505556b44202d0"
        );
        assert_eq!(
            extract_hash_from_run(fixture.path(), "outputs:withDeps").await,
            "c8be10745d2b5462452d49278e27b447c61a33c4049f025750d6ed9fb07a09fa"
        );
    }
}

mod target_scopes {
    use super::*;

    #[test]
    fn errors_for_deps_scope() {
        let fixture = create_sandbox("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("^:test")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn errors_for_self_scope() {
        let fixture = create_sandbox("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("~:test")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn supports_all_scope() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg(":all")
            .assert();
        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("targetScopeA:all").eval(&output));
        assert!(predicate::str::contains("targetScopeB:all").eval(&output));
        assert!(predicate::str::contains("targetScopeC:all").eval(&output));
        assert!(predicate::str::contains("Tasks: 3 completed").eval(&output));
    }

    #[test]
    fn supports_deps_scope_in_task() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("targetScopeA:deps")
            .assert();
        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("targetScopeA:deps").eval(&output));
        assert!(predicate::str::contains("depsA:standard").eval(&output));
        assert!(predicate::str::contains("depsB:standard").eval(&output));
        assert!(predicate::str::contains("depsC:standard").eval(&output));
        assert!(predicate::str::contains("Tasks: 4 completed").eval(&output));
    }

    #[test]
    fn supports_self_scope_in_task() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
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

mod outputs {
    use super::*;

    #[tokio::test]
    async fn errors_if_output_missing() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("outputs:missingOutput")
            .assert();

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("Target outputs:missingOutput defines the output unknown, but this output does not exist after being ran.").eval(&output));
    }

    #[tokio::test]
    async fn doesnt_cache_if_cache_disabled() {
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("run")
            .arg("outputs:noCache")
            .assert();

        let hash = extract_hash_from_run(fixture.path(), "outputs:noCache").await;

        assert_eq!(hash, "");

        // we cant assert the filesystem since the hash is empty!
    }

    #[tokio::test]
    async fn caches_single_file() {
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("run")
            .arg("outputs:generateFile")
            .assert();

        let hash = extract_hash_from_run(fixture.path(), "outputs:generateFile").await;

        // hash
        assert!(fixture
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{}.json", hash))
            .exists());

        // outputs
        assert!(fixture
            .path()
            .join(".moon/cache/outputs")
            .join(format!("{}.tar.gz", hash))
            .exists());
    }

    #[tokio::test]
    async fn caches_multiple_files() {
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("run")
            .arg("outputs:generateFiles")
            .assert();

        let hash = extract_hash_from_run(fixture.path(), "outputs:generateFiles").await;

        // hash
        assert!(fixture
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{}.json", hash))
            .exists());

        // outputs
        assert!(fixture
            .path()
            .join(".moon/cache/outputs")
            .join(format!("{}.tar.gz", hash))
            .exists());
    }

    #[tokio::test]
    async fn caches_single_folder() {
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("run")
            .arg("outputs:generateFolder")
            .assert();

        let hash = extract_hash_from_run(fixture.path(), "outputs:generateFolder").await;

        // hash
        assert!(fixture
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{}.json", hash))
            .exists());

        // outputs
        assert!(fixture
            .path()
            .join(".moon/cache/outputs")
            .join(format!("{}.tar.gz", hash))
            .exists());
    }

    #[tokio::test]
    async fn caches_multiple_folders() {
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("run")
            .arg("outputs:generateFolders")
            .assert();

        let hash = extract_hash_from_run(fixture.path(), "outputs:generateFolders").await;

        // hash
        assert!(fixture
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{}.json", hash))
            .exists());

        // outputs
        assert!(fixture
            .path()
            .join(".moon/cache/outputs")
            .join(format!("{}.tar.gz", hash))
            .exists());
    }

    #[tokio::test]
    async fn caches_both_file_and_folder() {
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("run")
            .arg("outputs:generateFileAndFolder")
            .assert();

        let hash = extract_hash_from_run(fixture.path(), "outputs:generateFileAndFolder").await;

        // hash
        assert!(fixture
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{}.json", hash))
            .exists());

        // outputs
        assert!(fixture
            .path()
            .join(".moon/cache/outputs")
            .join(format!("{}.tar.gz", hash))
            .exists());
    }

    #[tokio::test]
    async fn caches_output_logs_in_tarball() {
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("run")
            .arg("outputs:generateFile")
            .assert();

        let hash = extract_hash_from_run(fixture.path(), "outputs:generateFile").await;
        let tarball = fixture
            .path()
            .join(".moon/cache/outputs")
            .join(format!("{}.tar.gz", hash));
        let dir = fixture.path().join(".moon/cache/outputs").join(hash);

        moon_archive::untar(tarball, &dir, None).unwrap();

        assert!(dir.join("stdout.log").exists());
        assert!(dir.join("stderr.log").exists());
    }

    mod hydration {
        use super::*;
        use pretty_assertions::assert_eq;

        #[tokio::test]
        async fn reuses_cache_from_previous_run() {
            let fixture = create_sandbox_with_git("cases");

            let assert1 = create_moon_command(fixture.path())
                .arg("run")
                .arg("outputs:generateFileAndFolder")
                .assert();

            let hash1 =
                extract_hash_from_run(fixture.path(), "outputs:generateFileAndFolder").await;

            let assert2 = create_moon_command(fixture.path())
                .arg("run")
                .arg("outputs:generateFileAndFolder")
                .assert();

            let hash2 =
                extract_hash_from_run(fixture.path(), "outputs:generateFileAndFolder").await;

            assert_eq!(hash1, hash2);
            assert_snapshot!(get_assert_output(&assert1));
            assert_snapshot!(get_assert_output(&assert2));
        }

        #[tokio::test]
        async fn doesnt_keep_output_logs_in_project() {
            let fixture = create_sandbox_with_git("cases");

            create_moon_command(fixture.path())
                .arg("run")
                .arg("outputs:generateFileAndFolder")
                .assert();

            create_moon_command(fixture.path())
                .arg("run")
                .arg("outputs:generateFileAndFolder")
                .assert();

            assert!(!fixture.path().join("outputs/stdout.log").exists());
            assert!(!fixture.path().join("outputs/stderr.log").exists());
        }

        #[tokio::test]
        async fn hydrates_missing_outputs_from_previous_run() {
            let fixture = create_sandbox_with_git("cases");

            create_moon_command(fixture.path())
                .arg("run")
                .arg("outputs:generateFileAndFolder")
                .assert();

            // Remove outputs
            fs::remove_dir_all(fixture.path().join("outputs/esm")).unwrap();
            fs::remove_dir_all(fixture.path().join("outputs/lib")).unwrap();

            assert!(!fixture.path().join("outputs/esm").exists());
            assert!(!fixture.path().join("outputs/lib").exists());

            create_moon_command(fixture.path())
                .arg("run")
                .arg("outputs:generateFileAndFolder")
                .assert();

            // Outputs should come back
            assert!(fixture.path().join("outputs/esm").exists());
            assert!(fixture.path().join("outputs/lib").exists());
        }

        #[tokio::test]
        async fn hydrates_with_a_different_hash_cache() {
            let fixture = create_sandbox_with_git("cases");

            create_moon_command(fixture.path())
                .arg("run")
                .arg("outputs:generateFileAndFolder")
                .assert();

            let hash1 =
                extract_hash_from_run(fixture.path(), "outputs:generateFileAndFolder").await;
            let contents1 = fs::read_to_string(fixture.path().join("outputs/lib/one.js")).unwrap();

            // Create a file to trigger an inputs change
            fs::write(fixture.path().join("outputs/trigger.js"), "").unwrap();

            create_moon_command(fixture.path())
                .arg("run")
                .arg("outputs:generateFileAndFolder")
                .assert();

            let hash2 =
                extract_hash_from_run(fixture.path(), "outputs:generateFileAndFolder").await;
            let contents2 = fs::read_to_string(fixture.path().join("outputs/lib/one.js")).unwrap();

            // Hashes and contents should be different!
            assert_ne!(hash1, hash2);
            assert_ne!(contents1, contents2);

            // Remove outputs
            fs::remove_dir_all(fixture.path().join("outputs/esm")).unwrap();
            fs::remove_dir_all(fixture.path().join("outputs/lib")).unwrap();

            assert!(!fixture.path().join("outputs/esm").exists());
            assert!(!fixture.path().join("outputs/lib").exists());

            // Remove the trigger file
            fs::remove_file(fixture.path().join("outputs/trigger.js")).unwrap();

            create_moon_command(fixture.path())
                .arg("run")
                .arg("outputs:generateFileAndFolder")
                .assert();

            let hash3 =
                extract_hash_from_run(fixture.path(), "outputs:generateFileAndFolder").await;
            let contents3 = fs::read_to_string(fixture.path().join("outputs/lib/one.js")).unwrap();

            // Hashes and contents should match the original!
            assert_eq!(hash1, hash3);
            assert_eq!(contents1, contents3);
            assert_ne!(contents2, contents3);

            // Outputs should come back
            assert!(fixture.path().join("outputs/esm").exists());
            assert!(fixture.path().join("outputs/lib").exists());
        }
    }
}

mod noop {
    use super::*;

    #[test]
    fn runs_noop() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("noop:noop")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn runs_noop_deps() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("noop:noopWithDeps")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}

mod root_level {
    use super::*;

    #[test]
    fn runs_a_task() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("root:oneOff")
            .assert();

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("root one off").eval(&output));
    }
}

mod output_styles {
    use super::*;

    #[test]
    fn buffer() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("outputStyles:bufferPrimary")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn buffer_on_failure_when_success() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("outputStyles:bufferFailurePassPrimary")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[cfg(not(windows))] // Different path output in snapshot
    #[test]
    fn buffer_on_failure_when_failure() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("outputStyles:bufferFailureFailPrimary")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn hash() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("outputStyles:hashPrimary")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn none() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("outputStyles:nonePrimary")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn stream() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("outputStyles:streamPrimary")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}

mod reports {
    use super::*;

    #[test]
    fn doesnt_create_a_report_by_default() {
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("run")
            .arg("base:base")
            .assert();

        assert!(!fixture.path().join(".moon/cache/runReport.json").exists());
    }

    #[test]
    fn creates_report_when_option_passed() {
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("run")
            .arg("base:base")
            .arg("--report")
            .assert();

        assert!(fixture.path().join(".moon/cache/runReport.json").exists());
    }
}

mod affected {
    use super::*;
    use moon_utils::test::run_git_command;

    #[test]
    fn doesnt_run_if_not_affected() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("files:noop")
            .arg("--affected")
            .assert();

        let output = get_assert_output(&assert);

        assert!(
            predicate::str::contains("Target(s) files:noop not affected by touched files")
                .eval(&output)
        );
    }

    #[test]
    fn runs_if_affected() {
        let fixture = create_sandbox_with_git("cases");

        fs::write(fixture.path().join("files/other.txt"), "").unwrap();

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("files:noop")
            .arg("--affected")
            .assert();

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
    }

    #[test]
    fn doesnt_run_if_affected_but_wrong_status() {
        let fixture = create_sandbox_with_git("cases");

        fs::write(fixture.path().join("files/other.txt"), "").unwrap();

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("files:noop")
            .arg("--affected")
            .arg("--status")
            .arg("deleted")
            .assert();

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains(
            "Target(s) files:noop not affected by touched files (using status deleted)"
        )
        .eval(&output));
    }

    #[test]
    fn handles_untracked() {
        let fixture = create_sandbox_with_git("cases");

        fs::write(fixture.path().join("files/other.txt"), "").unwrap();

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("files:noop")
            .arg("--affected")
            .arg("--status")
            .arg("untracked")
            .assert();

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
    }

    #[test]
    fn handles_added() {
        let fixture = create_sandbox_with_git("cases");

        fs::write(fixture.path().join("files/other.txt"), "").unwrap();

        run_git_command(fixture.path(), |cmd| {
            cmd.args(["add", "files/other.txt"]);
        });

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("files:noop")
            .arg("--affected")
            .arg("--status")
            .arg("added")
            .assert();

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
    }

    #[test]
    fn handles_modified() {
        let fixture = create_sandbox_with_git("cases");

        fs::write(fixture.path().join("files/file.txt"), "modified").unwrap();

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("files:noop")
            .arg("--affected")
            .arg("--status")
            .arg("modified")
            .assert();

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
    }

    #[test]
    fn handles_deleted() {
        let fixture = create_sandbox_with_git("cases");

        fs::remove_file(fixture.path().join("files/file.txt")).unwrap();

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("files:noop")
            .arg("--affected")
            .arg("--status")
            .arg("deleted")
            .assert();

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
    }

    mod root_level {
        use super::*;

        #[test]
        fn doesnt_run_if_not_affected() {
            let fixture = create_sandbox_with_git("cases");

            let assert = create_moon_command(fixture.path())
                .arg("run")
                .arg("root:noop")
                .arg("--affected")
                .assert();

            let output = get_assert_output(&assert);

            assert!(
                predicate::str::contains("Target(s) root:noop not affected by touched files")
                    .eval(&output)
            );
        }

        #[test]
        fn runs_if_affected() {
            let fixture = create_sandbox_with_git("cases");

            fs::write(fixture.path().join("tsconfig.json"), "{}").unwrap();

            let assert = create_moon_command(fixture.path())
                .arg("run")
                .arg("root:noop")
                .arg("--affected")
                .assert();

            let output = get_assert_output(&assert);

            assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
        }

        #[test]
        fn doesnt_run_if_affected_but_wrong_status() {
            let fixture = create_sandbox_with_git("cases");

            fs::write(fixture.path().join("tsconfig.json"), "{}").unwrap();

            let assert = create_moon_command(fixture.path())
                .arg("run")
                .arg("root:noop")
                .arg("--affected")
                .arg("--status")
                .arg("deleted")
                .assert();

            let output = get_assert_output(&assert);

            assert!(predicate::str::contains(
                "Target(s) root:noop not affected by touched files (using status deleted)"
            )
            .eval(&output));
        }
    }
}
