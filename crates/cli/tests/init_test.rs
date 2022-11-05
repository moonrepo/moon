use moon_constants::{CONFIG_GLOBAL_PROJECT_FILENAME, CONFIG_WORKSPACE_FILENAME};
use moon_utils::test::{
    create_moon_command, create_sandbox, create_sandbox_with_git, run_git_command,
};
use predicates::prelude::*;
use std::fs;

#[test]
fn creates_files_in_dest() {
    let fixture = create_sandbox("init-sandbox");
    let root = fixture.path();
    let workspace_config = root.join(".moon").join(CONFIG_WORKSPACE_FILENAME);
    let project_config = root.join(".moon").join(CONFIG_GLOBAL_PROJECT_FILENAME);
    let gitignore = root.join(".gitignore");

    assert!(!workspace_config.exists());
    assert!(!project_config.exists());
    assert!(!gitignore.exists());

    let assert = create_moon_command(root)
        .arg("init")
        .arg("--yes")
        .arg(root)
        .assert();

    assert.success().code(0).stdout(predicate::str::contains(
        "moon has successfully been initialized in",
    ));

    assert!(workspace_config.exists());
    assert!(project_config.exists());
    assert!(gitignore.exists());
}

#[test]
fn creates_workspace_config_from_template() {
    let fixture = create_sandbox("init-sandbox");
    let root = fixture.path();
    let workspace_config = root.join(".moon").join("workspace.yml");

    create_moon_command(root)
        .arg("init")
        .arg("--yes")
        .arg(root)
        .assert();

    assert!(
        predicate::str::contains("https://moonrepo.dev/schemas/workspace.json")
            .eval(&fs::read_to_string(workspace_config).unwrap())
    );
}

#[test]
fn creates_project_config_from_template() {
    let fixture = create_sandbox("init-sandbox");
    let root = fixture.path();
    let project_config = root
        .join(".moon")
        .join(moon_constants::CONFIG_GLOBAL_PROJECT_FILENAME);

    create_moon_command(root)
        .arg("init")
        .arg("--yes")
        .arg(root)
        .assert();

    assert!(
        predicate::str::contains("https://moonrepo.dev/schemas/global-project.json")
            .eval(&fs::read_to_string(project_config).unwrap())
    );
}

#[test]
fn creates_gitignore_file() {
    let fixture = create_sandbox("init-sandbox");
    let root = fixture.path();
    let gitignore = root.join(".gitignore");

    create_moon_command(root)
        .arg("init")
        .arg("--yes")
        .arg(root)
        .assert();

    assert_eq!(
        fs::read_to_string(gitignore).unwrap(),
        "\n# moon\n.moon/cache\n.moon/docker\n"
    );
}

#[test]
fn appends_existing_gitignore_file() {
    let fixture = create_sandbox("init-sandbox");
    let root = fixture.path();
    let gitignore = root.join(".gitignore");

    fs::write(&gitignore, "*.js\n*.log").unwrap();

    create_moon_command(root)
        .arg("init")
        .arg("--yes")
        .arg(root)
        .assert();

    assert_eq!(
        fs::read_to_string(gitignore).unwrap(),
        "*.js\n*.log\n# moon\n.moon/cache\n.moon/docker\n"
    );
}

#[test]
fn does_overwrite_existing_config_if_force_passed() {
    let fixture = create_sandbox("init-sandbox");
    let root = fixture.path();

    create_moon_command(root)
        .arg("init")
        .arg("--yes")
        .arg(root)
        .assert();

    // Run again
    let assert = create_moon_command(root)
        .arg("init")
        .arg("--yes")
        .arg(root)
        .arg("--force")
        .assert();

    assert.success().code(0).stdout(predicate::str::contains(
        "moon has successfully been initialized in",
    ));
}

mod vcs {
    use super::*;

    #[test]

    fn detects_git() {
        let fixture = create_sandbox_with_git("init-sandbox");
        let root = fixture.path();
        let workspace_config = root.join(".moon").join("workspace.yml");

        // Checkout a new branch
        run_git_command(root, |cmd| {
            cmd.args(["checkout", "-b", "fixtures-test"]);
        });

        create_moon_command(root)
            .arg("init")
            .arg("--yes")
            .arg(root)
            .assert();

        let content = fs::read_to_string(workspace_config).unwrap();

        assert!(predicate::str::contains("manager: 'git'").eval(&content));
        assert!(predicate::str::contains("defaultBranch: 'fixtures-test'").eval(&content));
    }
}
