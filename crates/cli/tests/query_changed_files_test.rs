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
    fn can_supply_multi_status() {
        let sandbox = create_query_sandbox();

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
