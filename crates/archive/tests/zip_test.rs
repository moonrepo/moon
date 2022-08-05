use moon_archive::{unzip, zip};
use moon_utils::string_vec;
use moon_utils::test::create_sandbox;
use std::fs;
use std::path::Path;

fn file_contents_match(a: &Path, b: &Path) -> bool {
    fs::read_to_string(a).unwrap() == fs::read_to_string(b).unwrap()
}

#[test]
fn zips_file() {
    let fixture = create_sandbox("archives");

    // Pack
    let input = fixture.path();
    let archive = fixture.path().join("out.zip");

    zip(&input, &string_vec!["file.txt"], &archive, None).unwrap();

    assert!(archive.exists());
    assert_ne!(archive.metadata().unwrap().len(), 0);

    // Unpack
    let output = fixture.path().join("out");

    unzip(&archive, &output, None).unwrap();

    assert!(output.exists());
    assert!(output.join("file.txt").exists());

    // Compare
    assert!(file_contents_match(
        &input.join("file.txt"),
        &output.join("file.txt")
    ));
}

#[test]
fn zips_file_with_prefix() {
    let fixture = create_sandbox("archives");

    // Pack
    let input = fixture.path().join("file.txt");
    let archive = fixture.path().join("out.zip");

    zip(&input, &archive, Some("some/prefix")).unwrap();

    assert!(archive.exists());
    assert_ne!(archive.metadata().unwrap().len(), 0);

    // Unpack
    let output = fixture.path().join("out");

    unzip(&archive, &output, None).unwrap();

    assert!(output.exists());
    assert!(output.join("some/prefix/file.txt").exists());

    // Compare
    assert!(file_contents_match(
        &input,
        &output.join("some/prefix/file.txt")
    ));
}

#[test]
fn zips_file_with_prefix_thats_removed() {
    let fixture = create_sandbox("archives");

    // Pack
    let input = fixture.path().join("file.txt");
    let archive = fixture.path().join("out.zip");

    zip(&input, &archive, Some("some/prefix")).unwrap();

    assert!(archive.exists());
    assert_ne!(archive.metadata().unwrap().len(), 0);

    // Unpack
    let output = fixture.path().join("out");

    unzip(&archive, &output, Some("some/prefix")).unwrap();

    assert!(output.exists());
    assert!(output.join("file.txt").exists());

    // Compare
    assert!(file_contents_match(&input, &output.join("file.txt")));
}

#[test]
fn zips_dir() {
    let fixture = create_sandbox("archives");

    // Pack
    let input = fixture.path().join("folder");
    let archive = fixture.path().join("out.zip");

    zip(&input, &archive, None).unwrap();

    assert!(archive.exists());
    assert_ne!(archive.metadata().unwrap().len(), 0);

    // Unpack
    let output = fixture.path().join("out");

    unzip(&archive, &output, None).unwrap();

    assert!(output.exists());
    assert!(output.join("file.js").exists());
    assert!(output.join("nested/other.js").exists());

    // Compare
    assert!(file_contents_match(
        &input.join("file.js"),
        &output.join("file.js")
    ));
    assert!(file_contents_match(
        &input.join("nested/other.js"),
        &output.join("nested/other.js")
    ));
}

#[test]
fn zips_dir_with_prefix() {
    let fixture = create_sandbox("archives");

    // Pack
    let input = fixture.path().join("folder");
    let archive = fixture.path().join("out.zip");

    zip(&input, &archive, Some("some/prefix")).unwrap();

    assert!(archive.exists());
    assert_ne!(archive.metadata().unwrap().len(), 0);

    // Unpack
    let output = fixture.path().join("out");

    unzip(&archive, &output, None).unwrap();

    assert!(output.exists());
    assert!(output.join("some/prefix/file.js").exists());
    assert!(output.join("some/prefix/nested/other.js").exists());

    // Compare
    assert!(file_contents_match(
        &input.join("file.js"),
        &output.join("some/prefix/file.js")
    ));
    assert!(file_contents_match(
        &input.join("nested/other.js"),
        &output.join("some/prefix/nested/other.js")
    ));
}

#[test]
fn zips_dir_with_prefix_thats_removed() {
    let fixture = create_sandbox("archives");

    // Pack
    let input = fixture.path().join("folder");
    let archive = fixture.path().join("out.zip");

    zip(&input, &archive, Some("some/prefix")).unwrap();

    assert!(archive.exists());
    assert_ne!(archive.metadata().unwrap().len(), 0);

    // Unpack
    let output = fixture.path().join("out");

    unzip(&archive, &output, Some("some/prefix")).unwrap();

    assert!(output.exists());
    assert!(output.join("file.js").exists());
    assert!(output.join("nested/other.js").exists());

    // Compare
    assert!(file_contents_match(
        &input.join("file.js"),
        &output.join("file.js")
    ));
    assert!(file_contents_match(
        &input.join("nested/other.js"),
        &output.join("nested/other.js")
    ));
}
