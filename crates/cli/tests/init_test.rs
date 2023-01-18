use moon_constants::{CONFIG_TASKS_FILENAME, CONFIG_WORKSPACE_FILENAME};
use moon_test_utils::{create_sandbox, predicates::prelude::*};
use std::fs;

#[test]
fn creates_files_in_dest() {
    let sandbox = create_sandbox("init-sandbox");
    let root = sandbox.path().to_path_buf();
    let workspace_config = root.join(".moon").join(CONFIG_WORKSPACE_FILENAME);
    let project_config = root.join(".moon").join(CONFIG_TASKS_FILENAME);
    let gitignore = root.join(".gitignore");

    assert!(!workspace_config.exists());
    assert!(!project_config.exists());
    assert!(!gitignore.exists());

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("init").arg("--yes").arg(root);
    });

    assert.success().code(0).stdout(predicate::str::contains(
        "moon has successfully been initialized in",
    ));

    assert!(workspace_config.exists());
    assert!(project_config.exists());
    assert!(gitignore.exists());
}

#[test]
fn doesnt_create_project_config_when_minimal() {
    let sandbox = create_sandbox("init-sandbox");
    let root = sandbox.path().to_path_buf();
    let project_config = root.join(".moon").join(CONFIG_TASKS_FILENAME);

    assert!(!project_config.exists());

    sandbox.run_moon(|cmd| {
        cmd.arg("init").arg("--yes").arg("--minimal").arg(root);
    });

    assert!(!project_config.exists());
}

#[test]
fn creates_workspace_config_from_template() {
    let sandbox = create_sandbox("init-sandbox");
    let root = sandbox.path().to_path_buf();
    let workspace_config = root.join(".moon").join("workspace.yml");

    sandbox.run_moon(|cmd| {
        cmd.arg("init").arg("--yes").arg(root);
    });

    assert!(
        predicate::str::contains("https://moonrepo.dev/schemas/workspace.json")
            .eval(&fs::read_to_string(workspace_config).unwrap())
    );
}

#[test]
fn creates_project_config_from_template() {
    let sandbox = create_sandbox("init-sandbox");
    let root = sandbox.path().to_path_buf();
    let project_config = root
        .join(".moon")
        .join(moon_constants::CONFIG_TASKS_FILENAME);

    sandbox.run_moon(|cmd| {
        cmd.arg("init").arg("--yes").arg(root);
    });

    assert!(
        predicate::str::contains("https://moonrepo.dev/schemas/tasks.json")
            .eval(&fs::read_to_string(project_config).unwrap())
    );
}

#[test]
fn creates_gitignore_file() {
    let sandbox = create_sandbox("init-sandbox");
    let root = sandbox.path().to_path_buf();
    let gitignore = root.join(".gitignore");

    sandbox.run_moon(|cmd| {
        cmd.arg("init").arg("--yes").arg(root);
    });

    assert_eq!(
        fs::read_to_string(gitignore).unwrap(),
        "\n# moon\n.moon/cache\n.moon/docker\n"
    );
}

#[test]
fn appends_existing_gitignore_file() {
    let sandbox = create_sandbox("init-sandbox");
    let root = sandbox.path().to_path_buf();

    sandbox.create_file(".gitignore", "*.js\n*.log");

    sandbox.run_moon(|cmd| {
        cmd.arg("init").arg("--yes").arg(&root);
    });

    assert_eq!(
        fs::read_to_string(root.join(".gitignore")).unwrap(),
        "*.js\n*.log\n# moon\n.moon/cache\n.moon/docker\n"
    );
}

#[test]
fn does_overwrite_existing_config_if_force_passed() {
    let sandbox = create_sandbox("init-sandbox");
    let root = sandbox.path().to_path_buf();

    sandbox.run_moon(|cmd| {
        cmd.arg("init").arg("--yes").arg(&root);
    });

    // Run again
    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("init").arg("--yes").arg(root).arg("--force");
    });

    assert.success().code(0).stdout(predicate::str::contains(
        "moon has successfully been initialized in",
    ));
}

mod vcs {
    use super::*;

    #[test]
    fn detects_git() {
        let sandbox = create_sandbox("init-sandbox");
        sandbox.enable_git();

        let root = sandbox.path().to_path_buf();
        let workspace_config = root.join(".moon").join("workspace.yml");

        // Checkout a new branch
        sandbox.run_git(|cmd| {
            cmd.args(["checkout", "-b", "sandboxs-test"]);
        });

        sandbox.run_moon(|cmd| {
            cmd.arg("init").arg("--yes").arg(root);
        });

        let content = fs::read_to_string(workspace_config).unwrap();

        assert!(predicate::str::contains("manager: 'git'").eval(&content));
        assert!(predicate::str::contains("defaultBranch: 'sandboxs-test'").eval(&content));
    }
}
