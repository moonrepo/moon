use moon_codeowners::CodeownersGenerator;
use moon_config::{ConfigLoader, VcsProvider};
use starbase_sandbox::{Sandbox, assert_snapshot, create_empty_sandbox, locate_fixture};
use std::fs;

fn load_generator(provider: VcsProvider) -> Sandbox {
    let sandbox = create_empty_sandbox();
    let config_loader = ConfigLoader::default();

    sandbox.create_file(
        ".moon/workspace.yml",
        fs::read_to_string(locate_fixture("workspace").join("workspace.yml")).unwrap(),
    );

    let mut generator = CodeownersGenerator::new(sandbox.path(), provider).unwrap();
    let workspace_config = config_loader.load_workspace_config(sandbox.path()).unwrap();

    generator
        .add_workspace_entries(&workspace_config.codeowners)
        .unwrap();

    for project_fixture in ["custom-groups", "list-paths", "map-paths", "no-paths"] {
        sandbox.create_file(
            format!("{project_fixture}/moon.yml"),
            fs::read_to_string(locate_fixture(project_fixture).join("moon.yml")).unwrap(),
        );

        let project_config = config_loader
            .load_project_config_from_source(sandbox.path(), project_fixture)
            .unwrap();

        generator
            .add_project_entry(
                project_fixture,
                project_fixture,
                &project_config.owners,
                &workspace_config.codeowners,
            )
            .unwrap();
    }

    generator.generate().unwrap();

    sandbox
}

mod codeowners {
    use super::*;

    #[test]
    fn generates_bitbucket() {
        let sandbox = load_generator(VcsProvider::Bitbucket);

        assert_snapshot!(fs::read_to_string(sandbox.path().join("CODEOWNERS")).unwrap());
    }

    #[test]
    fn generates_github() {
        let sandbox = load_generator(VcsProvider::GitHub);

        assert_snapshot!(fs::read_to_string(sandbox.path().join(".github/CODEOWNERS")).unwrap());
    }

    #[test]
    fn generates_gitlab() {
        let sandbox = load_generator(VcsProvider::GitLab);

        assert_snapshot!(fs::read_to_string(sandbox.path().join(".gitlab/CODEOWNERS")).unwrap());
    }

    #[test]
    fn generates_other() {
        let sandbox = load_generator(VcsProvider::Other);

        assert_snapshot!(fs::read_to_string(sandbox.path().join("CODEOWNERS")).unwrap());
    }
}
