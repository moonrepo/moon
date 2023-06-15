use moon_vcs2::{Git, Vcs};
use starbase_sandbox::create_sandbox;

mod local {
    use super::*;

    #[tokio::test]
    async fn bin_version() {
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::new(sandbox.path()).unwrap();

        assert_eq!(git.get_version().await.unwrap().major, 2);
    }

    #[tokio::test]
    async fn local_branch() {
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::new(sandbox.path()).unwrap();

        assert_eq!(git.get_local_branch().await.unwrap(), "master");
    }

    #[tokio::test]
    async fn local_branch_after_switching() {
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();
        sandbox.run_git(|cmd| {
            cmd.args(["checkout", "-b", "feature"]);
        });

        let git = Git::new(sandbox.path()).unwrap();

        assert_eq!(git.get_local_branch().await.unwrap(), "feature");
    }

    #[tokio::test]
    async fn local_revision() {
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::new(sandbox.path()).unwrap();

        // Hash changes every time, so check that it's not empty
        assert_ne!(git.get_local_branch_revision().await.unwrap(), "");
    }

    #[tokio::test]
    async fn default_branch() {
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::load(sandbox.path(), "main", &[]).unwrap();

        assert_eq!(git.get_default_branch().await.unwrap(), "main");
    }

    #[tokio::test]
    async fn default_revision() {
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::new(sandbox.path()).unwrap();

        // Hash changes every time, so check that it's not empty
        assert_ne!(git.get_default_branch_revision().await.unwrap(), "");
    }
}
