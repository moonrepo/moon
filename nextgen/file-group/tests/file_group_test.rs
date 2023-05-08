use moon_file_group::FileGroup;
use moon_path::RelativePathBuf;

#[test]
fn sets_patterns() {
    let file_group = FileGroup::new_with_source("id", "project", ["a", "*", "b", "**/*"]).unwrap();

    assert_eq!(
        file_group.files,
        vec![
            RelativePathBuf::from("project/a"),
            RelativePathBuf::from("project/b")
        ]
    );
    assert_eq!(
        file_group.globs,
        vec![
            RelativePathBuf::from("project/*"),
            RelativePathBuf::from("project/**/*")
        ]
    );
}
