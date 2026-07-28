mod utils;

use moon_app::queries::changed_files::QueryChangedFilesResult;
use moon_vcs::ChangedStatus;
use starbase_utils::json::serde_json;
use utils::{change_branch, change_files, create_query_sandbox};

mod query_changed_files {
    use super::*;

    #[test]
    fn returns_files() {
        let sandbox = create_query_sandbox();

        change_files(&sandbox, ["basic/file.txt"]);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("changed-files");
        });

        let json: QueryChangedFilesResult = serde_json::from_str(assert.stdout().trim()).unwrap();

        assert_eq!(
            json.files
                .into_iter()
                .filter(|file| !file.as_str().starts_with(".moon"))
                .collect::<Vec<_>>(),
            ["basic/file.txt"]
        )
    }

    #[test]
    fn can_change_options() {
        let sandbox = create_query_sandbox();

        change_branch(&sandbox, "branch");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("changed-files").args([
                "--base", "master", "--head", "branch", "--status", "deleted",
            ]);
        });

        let json: QueryChangedFilesResult = serde_json::from_str(assert.stdout().trim()).unwrap();

        assert_eq!(json.options.base.unwrap(), "master".to_string());
        assert_eq!(json.options.head.unwrap(), "branch".to_string());
        assert_eq!(json.options.status, vec![ChangedStatus::Deleted]);
    }

    #[test]
    fn excludes_local_index_with_explicit_head() {
        let sandbox = create_query_sandbox();

        sandbox.create_file("basic/committed.txt", "contents");

        sandbox.run_git(|cmd| {
            cmd.args(["add", "--all", "."]);
        });

        sandbox.run_git(|cmd| {
            cmd.args(["commit", "-m", "Commit"]);
        });

        // Should not appear in the revision to revision diff
        sandbox.create_file("basic/dirty.txt", "contents");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query")
                .arg("changed-files")
                .args(["--base", "HEAD~1", "--head", "HEAD"]);
        });

        let json: QueryChangedFilesResult = serde_json::from_str(assert.stdout().trim()).unwrap();

        assert_eq!(
            json.files.into_iter().collect::<Vec<_>>(),
            ["basic/committed.txt"]
        );
    }

    #[test]
    fn includes_local_index_without_explicit_head() {
        let sandbox = create_query_sandbox();

        sandbox.create_file("basic/committed.txt", "contents");

        sandbox.run_git(|cmd| {
            cmd.args(["add", "--all", "."]);
        });

        sandbox.run_git(|cmd| {
            cmd.args(["commit", "-m", "Commit"]);
        });

        sandbox.create_file("basic/dirty.txt", "contents");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query")
                .arg("changed-files")
                .args(["--base", "HEAD~1"]);
        });

        let json: QueryChangedFilesResult = serde_json::from_str(assert.stdout().trim()).unwrap();

        let mut files = json
            .files
            .into_iter()
            .filter(|file| !file.as_str().starts_with(".moon"))
            .collect::<Vec<_>>();
        files.sort();

        assert_eq!(files, ["basic/committed.txt", "basic/dirty.txt"]);
    }

    #[test]
    fn treats_empty_head_as_working_tree() {
        let sandbox = create_query_sandbox();

        sandbox.create_file("basic/committed.txt", "contents");

        sandbox.run_git(|cmd| {
            cmd.args(["add", "--all", "."]);
        });

        sandbox.run_git(|cmd| {
            cmd.args(["commit", "-m", "Commit"]);
        });

        sandbox.create_file("basic/dirty.txt", "contents");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query")
                .arg("changed-files")
                .args(["--base", "HEAD~1"])
                .env("MOON_HEAD", "");
        });

        let json: QueryChangedFilesResult = serde_json::from_str(assert.stdout().trim()).unwrap();

        let mut files = json
            .files
            .into_iter()
            .filter(|file| !file.as_str().starts_with(".moon"))
            .collect::<Vec<_>>();
        files.sort();

        assert_eq!(files, ["basic/committed.txt", "basic/dirty.txt"]);
    }

    #[test]
    fn empty_env_vars_dont_mask_explicit_args() {
        let sandbox = create_query_sandbox();

        sandbox.create_file("basic/committed.txt", "contents");

        sandbox.run_git(|cmd| {
            cmd.args(["add", "--all", "."]);
        });

        sandbox.run_git(|cmd| {
            cmd.args(["commit", "-m", "Commit"]);
        });

        sandbox.create_file("basic/dirty.txt", "contents");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query")
                .arg("changed-files")
                .args(["--base", "HEAD~1", "--head", "HEAD"])
                .env("MOON_BASE", "")
                .env("MOON_HEAD", "");
        });

        let json: QueryChangedFilesResult = serde_json::from_str(assert.stdout().trim()).unwrap();

        assert_eq!(
            json.files.into_iter().collect::<Vec<_>>(),
            ["basic/committed.txt"]
        );
    }

    #[test]
    fn can_supply_multi_status() {
        let sandbox = create_query_sandbox();

        change_branch(&sandbox, "branch");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("changed-files").args([
                "--status", "deleted", "--status", "added", "--status", "modified",
            ]);
        });

        let json: QueryChangedFilesResult = serde_json::from_str(assert.stdout().trim()).unwrap();

        assert_eq!(
            json.options.status,
            vec![
                ChangedStatus::Deleted,
                ChangedStatus::Added,
                ChangedStatus::Modified
            ]
        );
    }
}
