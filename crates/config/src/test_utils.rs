use crate::shapes::{FileInput, FileOutput, GlobInput, GlobOutput, Uri};

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

pub fn create_file_output(path: impl AsRef<str>) -> FileOutput {
    let path = path.as_ref();

    FileOutput::from_uri(
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
            format!("glob://{}", path.replace("?", "__QM__"))
        })
        .unwrap(),
    )
    .unwrap()
}

pub fn create_glob_output(path: impl AsRef<str>) -> GlobOutput {
    let path = path.as_ref();

    GlobOutput::from_uri(
        Uri::parse(if path.starts_with("glob://") {
            path.to_owned()
        } else {
            format!("glob://{}", path.replace("?", "__QM__"))
        })
        .unwrap(),
    )
    .unwrap()
}
