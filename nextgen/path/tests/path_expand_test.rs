use moon_path::{expand_to_workspace_relative, WorkspaceRelativePathBuf};

mod expand_workspace_relative {
    use super::*;

    #[test]
    fn handles_ws_relative_path() {
        let project_source = "project";

        assert_eq!(
            expand_to_workspace_relative("/src/main.rs", project_source),
            WorkspaceRelativePathBuf::from("src/main.rs")
        );

        assert_eq!(
            expand_to_workspace_relative("/./src/main.rs", project_source),
            WorkspaceRelativePathBuf::from("src/main.rs")
        );
    }

    #[test]
    fn handles_ws_relative_glob() {
        let project_source = "project";

        assert_eq!(
            expand_to_workspace_relative("/src/*.rs", project_source),
            WorkspaceRelativePathBuf::from("src/*.rs")
        );
        assert_eq!(
            expand_to_workspace_relative("/./src/*.rs", project_source),
            WorkspaceRelativePathBuf::from("src/*.rs")
        );
    }

    #[test]
    fn handles_ws_relative_glob_negation() {
        let project_source = "project";

        assert_eq!(
            expand_to_workspace_relative("/!src/*.rs", project_source),
            WorkspaceRelativePathBuf::from("!src/*.rs")
        );
        assert_eq!(
            expand_to_workspace_relative("!/src/*.rs", project_source),
            WorkspaceRelativePathBuf::from("!src/*.rs")
        );
    }

    #[test]
    fn handles_proj_relative_path() {
        let project_source = "project";

        assert_eq!(
            expand_to_workspace_relative("src/main.rs", project_source),
            WorkspaceRelativePathBuf::from("project/src/main.rs")
        );
        assert_eq!(
            expand_to_workspace_relative("./src/main.rs", project_source),
            WorkspaceRelativePathBuf::from("project/src/main.rs")
        );
    }

    #[test]
    fn handles_proj_relative_glob() {
        let project_source = "project";

        assert_eq!(
            expand_to_workspace_relative("src/*.rs", project_source),
            WorkspaceRelativePathBuf::from("project/src/*.rs")
        );
        assert_eq!(
            expand_to_workspace_relative("./src/*.rs", project_source),
            WorkspaceRelativePathBuf::from("project/src/*.rs")
        );
    }

    #[test]
    fn handles_proj_relative_glob_negation() {
        let project_source = "project";

        assert_eq!(
            expand_to_workspace_relative("!src/*.rs", project_source),
            WorkspaceRelativePathBuf::from("!project/src/*.rs")
        );
        assert_eq!(
            expand_to_workspace_relative("!./src/*.rs", project_source),
            WorkspaceRelativePathBuf::from("!project/src/*.rs")
        );
    }
}
