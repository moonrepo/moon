// Integration tests for `cache.sharedWorktreeCache`: when running from a git
// worktree, CAS blobs and manifests are shared through the base checkout's
// `.moon/cache` (or `$MOON_HOME/cache/shared` when the repository root has no
// `.moon` directory, e.g. bare-clone workflows), while states, hashes, and
// locks stay in the worktree's own cache.

use moon_test_utils::{create_moon_sandbox, predicates::prelude::*};

mod worktree_cache {
    use super::*;

    #[test]
    fn shares_cas_with_base_checkout_when_repo_root_has_moon_dir() {
        let sandbox = create_moon_sandbox("worktree-cache");
        sandbox.enable_git();
        sandbox.run_git(|cmd| {
            cmd.args(["worktree", "add", "wt"]);
        });

        // Prime the cache from the base checkout
        sandbox
            .run_bin(|cmd| {
                cmd.arg("exec").arg("app:build");
            })
            .success();

        assert!(sandbox.path().join(".moon/cache/manifests").exists());
        assert!(sandbox.path().join("app/out.txt").exists());

        // The worktree's first ever run should hydrate from the base
        // checkout's CAS, without executing the task
        let wt = sandbox.path().join("wt");
        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("exec").arg("app:build");
            cmd.current_dir(&wt);
        });

        assert.success().stdout(predicate::str::contains("cached"));

        assert!(wt.join("app/out.txt").exists());

        // The worktree keeps its own engine cache, but no CAS of its own
        assert!(wt.join(".moon/cache/states").exists());
        assert!(!wt.join(".moon/cache/manifests").exists());
        assert!(!wt.join(".moon/cache/blobs").exists());

        // And the home fallback was not used
        assert!(!sandbox.path().join(".moon/cache/shared").exists());
    }

    #[test]
    fn falls_back_to_moon_home_when_repo_root_has_no_moon_dir() {
        let sandbox = create_moon_sandbox("worktree-cache");
        sandbox.enable_git();

        // Simulate a bare-clone workflow, where the repository root
        // contains git data instead of a checkout
        sandbox.run_git(|cmd| {
            cmd.args(["clone", "--bare", ".", "bare-repo"]);
        });
        sandbox.run_git(|cmd| {
            cmd.args(["worktree", "add", "../wt-one", "master"])
                .current_dir(sandbox.path().join("bare-repo"));
        });
        sandbox.run_git(|cmd| {
            cmd.args(["worktree", "add", "-b", "two", "../wt-two", "master"])
                .current_dir(sandbox.path().join("bare-repo"));
        });

        // The first worktree misses and archives into `$MOON_HOME/cache/shared`
        // (`MOON_HOME` points at the sandbox's `.moon` directory in tests)
        sandbox
            .run_bin(|cmd| {
                cmd.arg("exec").arg("app:build");
                cmd.current_dir(sandbox.path().join("wt-one"));
            })
            .success()
            .stdout(predicate::str::contains("cached").not());

        assert!(sandbox.path().join(".moon/cache/shared/manifests").exists());
        assert!(!sandbox.path().join("wt-one/.moon/cache/manifests").exists());

        // The second worktree, on another branch, hydrates from it
        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("exec").arg("app:build");
            cmd.current_dir(sandbox.path().join("wt-two"));
        });

        assert.success().stdout(predicate::str::contains("cached"));

        assert!(sandbox.path().join("wt-two/app/out.txt").exists());
    }

    #[test]
    fn keeps_cas_local_when_setting_disabled() {
        let sandbox = create_moon_sandbox("worktree-cache");
        sandbox.enable_git();
        sandbox.run_git(|cmd| {
            cmd.args(["worktree", "add", "wt"]);
        });

        let wt = sandbox.path().join("wt");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("exec").arg("app:build");
                cmd.current_dir(&wt);
                cmd.env("MOON_CACHE_SHARED_WORKTREE_CACHE", "false");
            })
            .success();

        // The CAS was created in the worktree itself, not the base checkout
        assert!(wt.join(".moon/cache/manifests").exists());
        assert!(!sandbox.path().join(".moon/cache/manifests").exists());
        assert!(!sandbox.path().join(".moon/cache/shared").exists());
    }
}
