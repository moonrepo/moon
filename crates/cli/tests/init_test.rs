use insta::assert_snapshot;
use moon_cli::commands::init::{init, InheritProjectsAs, InitOptions};
use moon_utils::test::create_fixtures_sandbox;
use predicates::prelude::*;
use serial_test::serial;
use std::fs;

#[tokio::test]
async fn creates_files_in_dest() {
    let fixture = create_fixtures_sandbox("init-sandbox");
    let root = fixture.path();
    let workspace_config = root.join(".moon").join("workspace.yml");
    let project_config = root.join(".moon").join("project.yml");
    let gitignore = root.join(".gitignore");

    assert!(!workspace_config.exists());
    assert!(!project_config.exists());
    assert!(!gitignore.exists());

    init(
        root.to_str().unwrap(),
        InitOptions {
            yes: true,
            ..InitOptions::default()
        },
    )
    .await
    .unwrap();

    assert!(workspace_config.exists());
    assert!(project_config.exists());
    assert!(gitignore.exists());
}

#[tokio::test]
async fn creates_workspace_config_from_template() {
    let fixture = create_fixtures_sandbox("init-sandbox");
    let root = fixture.path();
    let workspace_config = root.join(".moon").join("workspace.yml");

    init(
        root.to_str().unwrap(),
        InitOptions {
            yes: true,
            ..InitOptions::default()
        },
    )
    .await
    .unwrap();

    assert!(
        predicate::str::contains("https://moonrepo.dev/schemas/workspace.json")
            .eval(&fs::read_to_string(workspace_config).unwrap())
    );
}

#[tokio::test]
async fn creates_project_config_from_template() {
    let fixture = create_fixtures_sandbox("init-sandbox");
    let root = fixture.path();
    let project_config = root.join(".moon").join("project.yml");

    init(
        root.to_str().unwrap(),
        InitOptions {
            yes: true,
            ..InitOptions::default()
        },
    )
    .await
    .unwrap();

    assert!(
        predicate::str::contains("https://moonrepo.dev/schemas/global-project.json")
            .eval(&fs::read_to_string(project_config).unwrap())
    );
}

#[tokio::test]
async fn creates_gitignore_file() {
    let fixture = create_fixtures_sandbox("init-sandbox");
    let root = fixture.path();
    let gitignore = root.join(".gitignore");

    init(
        root.to_str().unwrap(),
        InitOptions {
            yes: true,
            ..InitOptions::default()
        },
    )
    .await
    .unwrap();

    assert_eq!(
        fs::read_to_string(gitignore).unwrap(),
        "\n# Moon\n.moon/cache\n"
    );
}

#[tokio::test]
async fn appends_existing_gitignore_file() {
    let fixture = create_fixtures_sandbox("init-sandbox");
    let root = fixture.path();
    let gitignore = root.join(".gitignore");

    fs::write(&gitignore, "*.js\n*.log").unwrap();

    init(
        root.to_str().unwrap(),
        InitOptions {
            yes: true,
            ..InitOptions::default()
        },
    )
    .await
    .unwrap();

    assert_eq!(
        fs::read_to_string(gitignore).unwrap(),
        "*.js\n*.log\n# Moon\n.moon/cache\n"
    );
}

#[tokio::test]
async fn does_overwrite_existing_config_if_force_passed() {
    let fixture = create_fixtures_sandbox("init-sandbox");
    let root = fixture.path();

    init(
        root.to_str().unwrap(),
        InitOptions {
            yes: true,
            ..InitOptions::default()
        },
    )
    .await
    .unwrap();

    // Run again
    init(
        root.to_str().unwrap(),
        InitOptions {
            force: true,
            yes: true,
            ..InitOptions::default()
        },
    )
    .await
    .unwrap();
}

mod node {
    use super::*;

    #[tokio::test]
    #[serial]
    async fn infers_version_from_nvm() {
        let fixture = create_fixtures_sandbox("init-sandbox");
        let root = fixture.path();
        let workspace_config = root.join(".moon").join("workspace.yml");

        fs::write(&root.join(".nvmrc"), "1.2.3").unwrap();

        init(
            root.to_str().unwrap(),
            InitOptions {
                yes: true,
                ..InitOptions::default()
            },
        )
        .await
        .unwrap();

        assert!(predicate::str::contains("version: '1.2.3'")
            .eval(&fs::read_to_string(workspace_config).unwrap()));
    }

    #[tokio::test]
    #[serial]
    async fn infers_version_from_nodenv() {
        let fixture = create_fixtures_sandbox("init-sandbox");
        let root = fixture.path();
        let workspace_config = root.join(".moon").join("workspace.yml");

        fs::write(&root.join(".node-version"), "1.2.3").unwrap();

        init(
            root.to_str().unwrap(),
            InitOptions {
                yes: true,
                ..InitOptions::default()
            },
        )
        .await
        .unwrap();

        assert!(predicate::str::contains("version: '1.2.3'")
            .eval(&fs::read_to_string(workspace_config).unwrap()));
    }

    #[tokio::test]
    #[serial]
    async fn infers_projects_from_workspaces() {
        let fixture = create_fixtures_sandbox("init-sandbox");
        let root = fixture.path();
        let workspace_config = root.join(".moon").join("workspace.yml");

        fs::create_dir_all(root.join("packages").join("foo")).unwrap();
        fs::write(&root.join("packages").join("foo").join("README"), "Hello").unwrap();

        fs::create_dir_all(root.join("app")).unwrap();
        fs::write(&root.join("app").join("README"), "World").unwrap();

        fs::write(
            &root.join("package.json"),
            r#"{"workspaces": ["packages/*", "app"] }"#,
        )
        .unwrap();

        init(
            root.to_str().unwrap(),
            InitOptions {
                inherit_projects: InheritProjectsAs::ProjectsMap,
                yes: true,
                ..InitOptions::default()
            },
        )
        .await
        .unwrap();

        assert_snapshot!(fs::read_to_string(workspace_config).unwrap());
    }

    #[tokio::test]
    #[serial]
    async fn infers_projects_from_workspaces_expanded() {
        let fixture = create_fixtures_sandbox("init-sandbox");
        let root = fixture.path();
        let workspace_config = root.join(".moon").join("workspace.yml");

        fs::create_dir_all(root.join("packages").join("bar")).unwrap();
        fs::write(&root.join("packages").join("bar").join("README"), "Hello").unwrap();

        fs::create_dir_all(root.join("app")).unwrap();
        fs::write(&root.join("app").join("README"), "World").unwrap();

        fs::write(
            &root.join("package.json"),
            r#"{"workspaces": { "packages": ["packages/*", "app"] }}"#,
        )
        .unwrap();

        init(
            root.to_str().unwrap(),
            InitOptions {
                inherit_projects: InheritProjectsAs::ProjectsMap,
                yes: true,
                ..InitOptions::default()
            },
        )
        .await
        .unwrap();

        assert_snapshot!(fs::read_to_string(workspace_config).unwrap());
    }

    #[tokio::test]
    #[serial]
    async fn infers_globs_from_workspaces() {
        let fixture = create_fixtures_sandbox("init-sandbox");
        let root = fixture.path();
        let workspace_config = root.join(".moon").join("workspace.yml");

        fs::create_dir_all(root.join("packages").join("foo")).unwrap();
        fs::write(&root.join("packages").join("foo").join("README"), "Hello").unwrap();

        fs::create_dir_all(root.join("app")).unwrap();
        fs::write(&root.join("app").join("README"), "World").unwrap();

        fs::write(
            &root.join("package.json"),
            r#"{"workspaces": ["packages/*", "app"] }"#,
        )
        .unwrap();

        init(
            root.to_str().unwrap(),
            InitOptions {
                inherit_projects: InheritProjectsAs::GlobsList,
                yes: true,
                ..InitOptions::default()
            },
        )
        .await
        .unwrap();

        assert_snapshot!(fs::read_to_string(workspace_config).unwrap());
    }

    #[tokio::test]
    #[serial]
    async fn infers_globs_from_workspaces_expanded() {
        let fixture = create_fixtures_sandbox("init-sandbox");
        let root = fixture.path();
        let workspace_config = root.join(".moon").join("workspace.yml");

        fs::create_dir_all(root.join("packages").join("bar")).unwrap();
        fs::write(&root.join("packages").join("bar").join("README"), "Hello").unwrap();

        fs::create_dir_all(root.join("app")).unwrap();
        fs::write(&root.join("app").join("README"), "World").unwrap();

        fs::write(
            &root.join("package.json"),
            r#"{"workspaces": { "packages": ["packages/*", "app"] }}"#,
        )
        .unwrap();

        init(
            root.to_str().unwrap(),
            InitOptions {
                inherit_projects: InheritProjectsAs::GlobsList,
                yes: true,
                ..InitOptions::default()
            },
        )
        .await
        .unwrap();

        assert_snapshot!(fs::read_to_string(workspace_config).unwrap());
    }

    mod package_manager {
        use super::*;

        #[tokio::test]
        #[serial]
        async fn infers_npm() {
            let fixture = create_fixtures_sandbox("init-sandbox");
            let root = fixture.path();
            let workspace_config = root.join(".moon").join("workspace.yml");

            fs::write(&root.join("package-lock.json"), "").unwrap();

            init(
                root.to_str().unwrap(),
                InitOptions {
                    yes: true,
                    ..InitOptions::default()
                },
            )
            .await
            .unwrap();

            assert!(predicate::str::contains("packageManager: 'npm'")
                .eval(&fs::read_to_string(workspace_config).unwrap()));
        }

        #[tokio::test]
        #[serial]
        async fn infers_npm_from_package() {
            let fixture = create_fixtures_sandbox("init-sandbox");
            let root = fixture.path();
            let workspace_config = root.join(".moon").join("workspace.yml");

            fs::write(
                &root.join("package.json"),
                r#"{"packageManager":"npm@4.5.6"}"#,
            )
            .unwrap();

            init(
                root.to_str().unwrap(),
                InitOptions {
                    yes: true,
                    ..InitOptions::default()
                },
            )
            .await
            .unwrap();

            let content = fs::read_to_string(workspace_config).unwrap();

            assert!(predicate::str::contains("packageManager: 'npm'").eval(&content));
            assert!(predicate::str::contains("version: '4.5.6'").eval(&content));
        }

        #[tokio::test]
        #[serial]
        async fn infers_pnpm() {
            let fixture = create_fixtures_sandbox("init-sandbox");
            let root = fixture.path();
            let workspace_config = root.join(".moon").join("workspace.yml");

            fs::write(&root.join("pnpm-lock.yaml"), "").unwrap();

            init(
                root.to_str().unwrap(),
                InitOptions {
                    yes: true,
                    ..InitOptions::default()
                },
            )
            .await
            .unwrap();

            assert!(predicate::str::contains("packageManager: 'pnpm'")
                .eval(&fs::read_to_string(workspace_config).unwrap()));
        }

        #[tokio::test]
        #[serial]
        async fn infers_pnpm_from_package() {
            let fixture = create_fixtures_sandbox("init-sandbox");
            let root = fixture.path();
            let workspace_config = root.join(".moon").join("workspace.yml");

            fs::write(
                &root.join("package.json"),
                r#"{"packageManager":"pnpm@4.5.6"}"#,
            )
            .unwrap();

            init(
                root.to_str().unwrap(),
                InitOptions {
                    yes: true,
                    ..InitOptions::default()
                },
            )
            .await
            .unwrap();

            let content = fs::read_to_string(workspace_config).unwrap();

            assert!(predicate::str::contains("packageManager: 'pnpm'").eval(&content));
            assert!(predicate::str::contains("version: '4.5.6'").eval(&content));
        }

        #[tokio::test]
        #[serial]
        async fn infers_yarn() {
            let fixture = create_fixtures_sandbox("init-sandbox");
            let root = fixture.path();
            let workspace_config = root.join(".moon").join("workspace.yml");

            fs::write(&root.join("yarn.lock"), "").unwrap();

            init(
                root.to_str().unwrap(),
                InitOptions {
                    yes: true,
                    ..InitOptions::default()
                },
            )
            .await
            .unwrap();

            assert!(predicate::str::contains("packageManager: 'yarn'")
                .eval(&fs::read_to_string(workspace_config).unwrap()));
        }

        #[tokio::test]
        #[serial]
        async fn infers_yarn_from_package() {
            let fixture = create_fixtures_sandbox("init-sandbox");
            let root = fixture.path();
            let workspace_config = root.join(".moon").join("workspace.yml");

            fs::write(
                &root.join("package.json"),
                r#"{"packageManager":"yarn@4.5.6"}"#,
            )
            .unwrap();

            init(
                root.to_str().unwrap(),
                InitOptions {
                    yes: true,
                    ..InitOptions::default()
                },
            )
            .await
            .unwrap();

            let content = fs::read_to_string(workspace_config).unwrap();

            assert!(predicate::str::contains("packageManager: 'yarn'").eval(&content));
            assert!(predicate::str::contains("version: '4.5.6'").eval(&content));
        }
    }
}
