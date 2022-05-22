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

// #[test]
// #[serial]
// fn doesnt_overwrite_existing_config() {
//     let fixture = create_fixtures_sandbox("init-sandbox");
//     let root = fixture.path();

//     create_moon_command_in(root)
//         .arg("init")
//         .arg("--yes")
//         .arg(&root)
//         .assert();

//     // Run again
//     let assert = create_moon_command_in(root)
//         .arg("init")
//         .arg("--yes")
//         .arg(&root)
//         .assert();

//     assert.success().code(0).stdout(predicate::str::starts_with(
//         "Moon has already been initialized in",
//     ));
// }

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
