use moon_utils::path;
use std::path::PathBuf;

mod expand_workspace_relative {
    use super::*;

    #[test]
    fn handles_ws_relative_path() {
        let workspace_root = PathBuf::from("/workspace");
        let project_root = PathBuf::from("/workspace/project");

        assert_eq!(
            path::expand_to_workspace_relative("/src/main.rs", workspace_root, project_root),
            PathBuf::from("src/main.rs")
        );
    }

    #[test]
    fn handles_ws_relative_glob() {
        let workspace_root = PathBuf::from("/workspace");
        let project_root = PathBuf::from("/workspace/project");

        assert_eq!(
            path::expand_to_workspace_relative("/src/*.rs", workspace_root, project_root),
            PathBuf::from("src/*.rs")
        );
    }

    #[test]
    fn handles_ws_relative_glob_negation() {
        let workspace_root = PathBuf::from("/workspace");
        let project_root = PathBuf::from("/workspace/project");

        assert_eq!(
            path::expand_to_workspace_relative("/!src/*.rs", workspace_root, project_root),
            PathBuf::from("!src/*.rs")
        );
    }

    #[test]
    fn handles_proj_relative_path() {
        let workspace_root = PathBuf::from("/workspace");
        let project_root = PathBuf::from("/workspace/project");

        assert_eq!(
            path::expand_to_workspace_relative("src/main.rs", &workspace_root, &project_root),
            PathBuf::from("project/src/main.rs")
        );
        assert_eq!(
            path::expand_to_workspace_relative("./src/main.rs", workspace_root, project_root),
            PathBuf::from("project/src/main.rs")
        );
    }

    #[test]
    fn handles_proj_relative_glob() {
        let workspace_root = PathBuf::from("/workspace");
        let project_root = PathBuf::from("/workspace/project");

        assert_eq!(
            path::expand_to_workspace_relative("src/*.rs", &workspace_root, &project_root),
            PathBuf::from("project/src/*.rs")
        );
        assert_eq!(
            path::expand_to_workspace_relative("./src/*.rs", workspace_root, project_root),
            PathBuf::from("project/src/*.rs")
        );
    }

    #[test]
    fn handles_proj_relative_glob_negation() {
        let workspace_root = PathBuf::from("/workspace");
        let project_root = PathBuf::from("/workspace/project");

        assert_eq!(
            path::expand_to_workspace_relative("!src/*.rs", &workspace_root, &project_root),
            PathBuf::from("!project/src/*.rs")
        );
        assert_eq!(
            path::expand_to_workspace_relative("!./src/*.rs", workspace_root, project_root),
            PathBuf::from("!project/src/*.rs")
        );
    }
}
