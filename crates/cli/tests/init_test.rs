use insta::assert_snapshot;
use moon_utils::test::{create_fixtures_sandbox, create_moon_command_in};
use predicates::prelude::*;
use serial_test::serial;
use std::fs;

#[test]
#[serial]
fn creates_files_in_dest() {
    let fixture = create_fixtures_sandbox("init-sandbox");
    let root = fixture.path();
    let workspace_config = root.join(".moon").join("workspace.yml");
    let project_config = root.join(".moon").join("project.yml");
    let gitignore = root.join(".gitignore");

    assert!(!workspace_config.exists());
    assert!(!project_config.exists());
    assert!(!gitignore.exists());

    let assert = create_moon_command_in(root)
        .arg("init")
        .arg("--yes")
        .arg(&root)
        .assert();

    assert.success().code(0).stdout(predicate::str::starts_with(
        "Moon has successfully been initialized in",
    ));

    assert!(workspace_config.exists());
    assert!(project_config.exists());
    assert!(gitignore.exists());
}

#[test]
#[serial]
fn creates_workspace_config_from_template() {
    let fixture = create_fixtures_sandbox("init-sandbox");
    let root = fixture.path();
    let workspace_config = root.join(".moon").join("workspace.yml");

    create_moon_command_in(root)
        .arg("init")
        .arg("--yes")
        .arg(&root)
        .assert();

    assert!(
        predicate::str::contains("https://moonrepo.dev/schemas/workspace.json")
            .eval(&fs::read_to_string(workspace_config).unwrap())
    );
}

#[test]
#[serial]
fn creates_project_config_from_template() {
    let fixture = create_fixtures_sandbox("init-sandbox");
    let root = fixture.path();
    let project_config = root.join(".moon").join("project.yml");

    create_moon_command_in(root)
        .arg("init")
        .arg("--yes")
        .arg(&root)
        .assert();

    assert!(
        predicate::str::contains("https://moonrepo.dev/schemas/global-project.json")
            .eval(&fs::read_to_string(project_config).unwrap())
    );
}

#[test]
#[serial]
fn creates_gitignore_file() {
    let fixture = create_fixtures_sandbox("init-sandbox");
    let root = fixture.path();
    let gitignore = root.join(".gitignore");

    create_moon_command_in(root)
        .arg("init")
        .arg("--yes")
        .arg(&root)
        .assert();

    assert_eq!(
        fs::read_to_string(gitignore).unwrap(),
        "\n# Moon\n.moon/cache\n"
    );
}

#[test]
#[serial]
fn appends_existing_gitignore_file() {
    let fixture = create_fixtures_sandbox("init-sandbox");
    let root = fixture.path();
    let gitignore = root.join(".gitignore");

    fs::write(&gitignore, "*.js\n*.log").unwrap();

    create_moon_command_in(root)
        .arg("init")
        .arg("--yes")
        .arg(&root)
        .assert();

    assert_eq!(
        fs::read_to_string(gitignore).unwrap(),
        "*.js\n*.log\n# Moon\n.moon/cache\n"
    );
}

#[test]
#[serial]
fn does_overwrite_existing_config_if_force_passed() {
    let fixture = create_fixtures_sandbox("init-sandbox");
    let root = fixture.path();

    create_moon_command_in(root)
        .arg("init")
        .arg("--yes")
        .arg(&root)
        .assert();

    // Run again
    let assert = create_moon_command_in(root)
        .arg("init")
        .arg("--yes")
        .arg(&root)
        .arg("--force")
        .assert();

    assert.success().code(0).stdout(predicate::str::starts_with(
        "Moon has successfully been initialized in",
    ));
}

mod node {
    use super::*;

    #[test]
    #[serial]
    fn infers_version_from_nvm() {
        let fixture = create_fixtures_sandbox("init-sandbox");
        let root = fixture.path();
        let workspace_config = root.join(".moon").join("workspace.yml");

        fs::write(&root.join(".nvmrc"), "1.2.3").unwrap();

        create_moon_command_in(root)
            .arg("init")
            .arg("--yes")
            .arg(&root)
            .assert();

        assert!(predicate::str::contains("version: '1.2.3'")
            .eval(&fs::read_to_string(workspace_config).unwrap()));
    }

    #[test]
    #[serial]
    fn infers_version_from_nodenv() {
        let fixture = create_fixtures_sandbox("init-sandbox");
        let root = fixture.path();
        let workspace_config = root.join(".moon").join("workspace.yml");

        fs::write(&root.join(".node-version"), "1.2.3").unwrap();

        create_moon_command_in(root)
            .arg("init")
            .arg("--yes")
            .arg(&root)
            .assert();

        assert!(predicate::str::contains("version: '1.2.3'")
            .eval(&fs::read_to_string(workspace_config).unwrap()));
    }

    #[test]
    #[serial]
    fn infers_projects_from_workspaces() {
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

        create_moon_command_in(root)
            .arg("init")
            .arg("--yes")
            .arg("--inheritProjects")
            .arg("projects-map")
            .arg(&root)
            .assert();

        assert_snapshot!(fs::read_to_string(workspace_config).unwrap());
    }

    #[test]
    #[serial]
    fn infers_projects_from_workspaces_expanded() {
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

        create_moon_command_in(root)
            .arg("init")
            .arg("--yes")
            .arg("--inheritProjects")
            .arg("projects-map")
            .arg(&root)
            .assert();

        assert_snapshot!(fs::read_to_string(workspace_config).unwrap());
    }

    #[test]
    #[serial]
    fn infers_globs_from_workspaces() {
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

        create_moon_command_in(root)
            .arg("init")
            .arg("--yes")
            .arg("--inheritProjects")
            .arg("globs-list")
            .arg(&root)
            .assert();

        assert_snapshot!(fs::read_to_string(workspace_config).unwrap());
    }

    #[test]
    #[serial]
    fn infers_globs_from_workspaces_expanded() {
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

        create_moon_command_in(root)
            .arg("init")
            .arg("--yes")
            .arg("--inheritProjects")
            .arg("globs-list")
            .arg(&root)
            .assert();

        assert_snapshot!(fs::read_to_string(workspace_config).unwrap());
    }

    mod package_manager {
        use super::*;

        #[test]
        #[serial]
        fn infers_npm() {
            let fixture = create_fixtures_sandbox("init-sandbox");
            let root = fixture.path();
            let workspace_config = root.join(".moon").join("workspace.yml");

            fs::write(&root.join("package-lock.json"), "").unwrap();

            create_moon_command_in(root)
                .arg("init")
                .arg("--yes")
                .arg(&root)
                .assert();

            assert!(predicate::str::contains("packageManager: 'npm'")
                .eval(&fs::read_to_string(workspace_config).unwrap()));
        }

        #[test]
        #[serial]
        fn infers_npm_from_package() {
            let fixture = create_fixtures_sandbox("init-sandbox");
            let root = fixture.path();
            let workspace_config = root.join(".moon").join("workspace.yml");

            fs::write(
                &root.join("package.json"),
                r#"{"packageManager":"npm@4.5.6"}"#,
            )
            .unwrap();

            create_moon_command_in(root)
                .arg("init")
                .arg("--yes")
                .arg(&root)
                .assert();

            let content = fs::read_to_string(workspace_config).unwrap();

            assert!(predicate::str::contains("packageManager: 'npm'").eval(&content));
            assert!(predicate::str::contains("version: '4.5.6'").eval(&content));
        }

        #[test]
        #[serial]
        fn infers_pnpm() {
            let fixture = create_fixtures_sandbox("init-sandbox");
            let root = fixture.path();
            let workspace_config = root.join(".moon").join("workspace.yml");

            fs::write(&root.join("pnpm-lock.yaml"), "").unwrap();

            create_moon_command_in(root)
                .arg("init")
                .arg("--yes")
                .arg(&root)
                .assert();

            assert!(predicate::str::contains("packageManager: 'pnpm'")
                .eval(&fs::read_to_string(workspace_config).unwrap()));
        }

        #[test]
        #[serial]
        fn infers_pnpm_from_package() {
            let fixture = create_fixtures_sandbox("init-sandbox");
            let root = fixture.path();
            let workspace_config = root.join(".moon").join("workspace.yml");

            fs::write(
                &root.join("package.json"),
                r#"{"packageManager":"pnpm@4.5.6"}"#,
            )
            .unwrap();

            create_moon_command_in(root)
                .arg("init")
                .arg("--yes")
                .arg(&root)
                .assert();

            let content = fs::read_to_string(workspace_config).unwrap();

            assert!(predicate::str::contains("packageManager: 'pnpm'").eval(&content));
            assert!(predicate::str::contains("version: '4.5.6'").eval(&content));
        }

        #[test]
        #[serial]
        fn infers_yarn() {
            let fixture = create_fixtures_sandbox("init-sandbox");
            let root = fixture.path();
            let workspace_config = root.join(".moon").join("workspace.yml");

            fs::write(&root.join("yarn.lock"), "").unwrap();

            create_moon_command_in(root)
                .arg("init")
                .arg("--yes")
                .arg(&root)
                .assert();

            assert!(predicate::str::contains("packageManager: 'yarn'")
                .eval(&fs::read_to_string(workspace_config).unwrap()));
        }

        #[test]
        #[serial]
        fn infers_yarn_from_package() {
            let fixture = create_fixtures_sandbox("init-sandbox");
            let root = fixture.path();
            let workspace_config = root.join(".moon").join("workspace.yml");

            fs::write(
                &root.join("package.json"),
                r#"{"packageManager":"yarn@4.5.6"}"#,
            )
            .unwrap();

            create_moon_command_in(root)
                .arg("init")
                .arg("--yes")
                .arg(&root)
                .assert();

            let content = fs::read_to_string(workspace_config).unwrap();

            assert!(predicate::str::contains("packageManager: 'yarn'").eval(&content));
            assert!(predicate::str::contains("version: '4.5.6'").eval(&content));
        }
    }
}
