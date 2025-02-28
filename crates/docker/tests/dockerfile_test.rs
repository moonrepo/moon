use moon_common::Id;
use moon_docker::{GenerateDockerfileOptions, generate_dockerfile};
use moon_target::Target;
use starbase_sandbox::assert_snapshot;

fn create_options() -> GenerateDockerfileOptions {
    GenerateDockerfileOptions {
        project: Id::raw("app"),
        ..Default::default()
    }
}

mod dockerfile {
    use super::*;

    #[test]
    fn renders_defaults() {
        assert_snapshot!(generate_dockerfile(create_options()).unwrap());
    }

    #[test]
    fn disables_toolchain() {
        let mut options = create_options();
        options.disable_toolchain = true;

        assert_snapshot!(generate_dockerfile(options).unwrap());
    }

    #[test]
    fn with_tasks() {
        let mut options = create_options();
        options.build_task = Some(Target::parse("app:compile").unwrap());
        options.start_task = Some(Target::parse("app:serve").unwrap());

        assert_snapshot!(generate_dockerfile(options).unwrap());
    }

    #[test]
    fn with_prune() {
        let mut options = create_options();
        options.prune = true;
        options.build_task = Some(Target::parse("app:compile").unwrap());
        options.start_task = Some(Target::parse("app:serve").unwrap());

        assert_snapshot!(generate_dockerfile(options).unwrap());
    }
}
