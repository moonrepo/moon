use insta::assert_snapshot;
use moon_utils::test::{create_moon_command, create_sandbox, get_assert_output};
use predicates::prelude::*;
use std::fs;

fn get_path_safe_output(assert: &assert_cmd::assert::Assert) -> String {
    get_assert_output(assert).replace('\\', "/")
}

#[test]
fn creates_a_new_template() {
    let fixture = create_sandbox("generator");

    let assert = create_moon_command(fixture.path())
        .arg("generate")
        .arg("new-name")
        .arg("--template")
        .assert();
    let output = get_path_safe_output(&assert);

    assert!(predicate::str::contains("Created a new template new-name at").eval(&output));
    assert!(fixture.path().join("templates/new-name").exists());

    assert.success();
}

#[test]
fn generates_files_from_template() {
    let fixture = create_sandbox("generator");

    let assert = create_moon_command(fixture.path())
        .arg("generate")
        .arg("standard")
        .arg("./test")
        .assert();

    assert_snapshot!(get_path_safe_output(&assert));

    assert!(fixture.path().join("test").exists());
    assert!(fixture.path().join("test/file.ts").exists());
    assert!(fixture.path().join("test/folder/nested-file.ts").exists());
    assert!(!fixture.path().join("test/template.yml").exists());
}

#[test]
fn doesnt_generate_files_when_dryrun() {
    let fixture = create_sandbox("generator");

    let assert = create_moon_command(fixture.path())
        .arg("generate")
        .arg("standard")
        .arg("./test")
        .arg("--dry-run")
        .assert();

    assert_snapshot!(get_path_safe_output(&assert));

    assert!(!fixture.path().join("test").exists());
    assert!(!fixture.path().join("test/file.ts").exists());
    assert!(!fixture.path().join("test/folder/nested-file.ts").exists());
    assert!(!fixture.path().join("test/template.yml").exists());
}

#[test]
fn overwrites_existing_files_when_forced() {
    let fixture = create_sandbox("generator");

    create_moon_command(fixture.path())
        .arg("generate")
        .arg("standard")
        .arg("./test")
        .assert();

    let assert = create_moon_command(fixture.path())
        .arg("generate")
        .arg("standard")
        .arg("./test")
        .arg("--force")
        .assert();

    assert_snapshot!(get_path_safe_output(&assert));

    assert!(fixture.path().join("test").exists());
    assert!(fixture.path().join("test/file.ts").exists());
    assert!(fixture.path().join("test/folder/nested-file.ts").exists());
    assert!(!fixture.path().join("test/template.yml").exists());
}

#[test]
fn overwrites_existing_files_when_interpolated_path() {
    let fixture = create_sandbox("generator");

    create_moon_command(fixture.path())
        .arg("generate")
        .arg("vars")
        .arg("./test")
        .arg("--defaults")
        .assert();

    let assert = create_moon_command(fixture.path())
        .arg("generate")
        .arg("vars")
        .arg("./test")
        .arg("--defaults")
        .arg("--force")
        .assert();

    assert_snapshot!(get_path_safe_output(&assert));

    // file-[stringNotEmpty]-[number].txt
    assert!(fixture.path().join("./test/file-default-0.txt").exists());
}

#[test]
fn renders_and_interpolates_templates() {
    let fixture = create_sandbox("generator");

    let assert = create_moon_command(fixture.path())
        .arg("generate")
        .arg("vars")
        .arg("./test")
        .arg("--defaults")
        .assert();

    assert.success();

    assert_snapshot!(fs::read_to_string(fixture.path().join("./test/expressions.txt")).unwrap());
    assert_snapshot!(fs::read_to_string(fixture.path().join("./test/control.txt")).unwrap());
}

#[test]
fn interpolates_destination_path() {
    let fixture = create_sandbox("generator");

    let assert = create_moon_command(fixture.path())
        .arg("generate")
        .arg("vars")
        .arg("./test")
        .arg("--defaults")
        .assert();

    // Verify output paths are correct
    assert_snapshot!(get_path_safe_output(&assert));

    // file-[stringNotEmpty]-[number].txt
    assert!(fixture.path().join("./test/file-default-0.txt").exists());
}
