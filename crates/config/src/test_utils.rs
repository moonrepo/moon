use crate::shapes::{FileInput, GlobInput, Uri};

pub fn create_file_input(path: impl AsRef<str>) -> FileInput {
    let path = path.as_ref();

    FileInput::from_uri(
        Uri::parse(if path.starts_with("file://") {
            path.to_owned()
        } else {
            format!("file://{path}")
        })
        .unwrap(),
    )
    .unwrap()
}

pub fn create_glob_input(path: impl AsRef<str>) -> GlobInput {
    let path = path.as_ref();

    GlobInput::from_uri(
        Uri::parse(if path.starts_with("glob://") {
            path.to_owned()
        } else {
            format!("glob://{path}")
        })
        .unwrap(),
    )
    .unwrap()
}
