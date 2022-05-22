use moon_config::{load_global_project_config_template, load_workspace_config_template};
use moon_utils::test::{create_moon_command, get_fixtures_dir};
use predicates::prelude::*;
use serial_test::serial;
use std::fs;
use std::path::PathBuf;

fn cleanup_sandbox(root: PathBuf) {
    fs::remove_dir_all(root.join(".moon")).unwrap();
    fs::remove_file(root.join(".gitignore")).unwrap();
}

#[test]
#[serial]
fn creates_files_in_dest() {
    let root = get_fixtures_dir("init-sandbox");
    let workspace_config = root.join(".moon").join("workspace.yml");
    let project_config = root.join(".moon").join("project.yml");
    let gitignore = root.join(".gitignore");

    assert!(!workspace_config.exists());
    assert!(!project_config.exists());
    assert!(!gitignore.exists());

    let assert = create_moon_command("init-sandbox")
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

    cleanup_sandbox(root);
}

#[test]
#[serial]
fn creates_workspace_config_from_template() {
    let root = get_fixtures_dir("init-sandbox");
    let workspace_config = root.join(".moon").join("workspace.yml");

    create_moon_command("init-sandbox")
        .arg("init")
        .arg("--yes")
        .arg(&root)
        .assert();

    assert_eq!(
        fs::read_to_string(workspace_config).unwrap(),
        load_workspace_config_template()
    );

    cleanup_sandbox(root);
}

#[test]
#[serial]
fn creates_project_config_from_template() {
    let root = get_fixtures_dir("init-sandbox");
    let project_config = root.join(".moon").join("project.yml");

    create_moon_command("init-sandbox")
        .arg("init")
        .arg("--yes")
        .arg(&root)
        .assert();

    assert_eq!(
        fs::read_to_string(project_config).unwrap(),
        load_global_project_config_template()
    );

    cleanup_sandbox(root);
}

#[test]
#[serial]
fn creates_gitignore_file() {
    let root = get_fixtures_dir("init-sandbox");
    let gitignore = root.join(".gitignore");

    create_moon_command("init-sandbox")
        .arg("init")
        .arg("--yes")
        .arg(&root)
        .assert();

    assert_eq!(
        fs::read_to_string(gitignore).unwrap(),
        "\n# Moon\n.moon/cache\n"
    );

    cleanup_sandbox(root);
}

#[test]
#[serial]
fn appends_existing_gitignore_file() {
    let root = get_fixtures_dir("init-sandbox");
    let gitignore = root.join(".gitignore");

    fs::write(&gitignore, "*.js\n*.log").unwrap();

    create_moon_command("init-sandbox")
        .arg("init")
        .arg("--yes")
        .arg(&root)
        .assert();

    assert_eq!(
        fs::read_to_string(gitignore).unwrap(),
        "*.js\n*.log\n# Moon\n.moon/cache\n"
    );

    cleanup_sandbox(root);
}

#[test]
#[serial]
fn doesnt_overwrite_existing_config() {
    let root = get_fixtures_dir("init-sandbox");

    create_moon_command("init-sandbox")
        .arg("init")
        .arg("--yes")
        .arg(&root)
        .assert();

    // Run again
    let assert = create_moon_command("init-sandbox")
        .arg("init")
        .arg("--yes")
        .arg(&root)
        .assert();

    assert.success().code(0).stdout(predicate::str::starts_with(
        "Moon has already been initialized in",
    ));

    cleanup_sandbox(root);
}

#[test]
#[serial]
fn does_overwrite_existing_config_if_force_passed() {
    let root = get_fixtures_dir("init-sandbox");

    create_moon_command("init-sandbox")
        .arg("init")
        .arg("--yes")
        .arg(&root)
        .assert();

    // Run again
    let assert = create_moon_command("init-sandbox")
        .arg("init")
        .arg("--yes")
        .arg(&root)
        .arg("--force")
        .assert();

    assert.success().code(0).stdout(predicate::str::starts_with(
        "Moon has successfully been initialized in",
    ));

    cleanup_sandbox(root);
}
