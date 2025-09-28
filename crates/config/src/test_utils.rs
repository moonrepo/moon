use crate::shapes::{FileInput, FileOutput, GlobInput, GlobOutput, Uri};

fn create_uri(prefix: &str, path: impl AsRef<str>) -> Uri {
    let protocol = format!("{prefix}://");
    let path = path.as_ref();

    Uri::parse(if path.starts_with(&protocol) {
        path.to_owned()
    } else {
        format!(
            "{protocol}{}",
            if prefix == "glob" {
                path.replace("?", "__QM__")
            } else {
                path.to_owned()
            }
        )
    })
    .unwrap()
}

pub fn stub_file_input(path: impl AsRef<str>) -> FileInput {
    FileInput::from_uri(create_uri("file", path)).unwrap()
}

pub fn stub_file_output(path: impl AsRef<str>) -> FileOutput {
    FileOutput::from_uri(create_uri("file", path)).unwrap()
}

pub fn stub_glob_input(path: impl AsRef<str>) -> GlobInput {
    GlobInput::from_uri(create_uri("glob", path)).unwrap()
}

pub fn stub_glob_output(path: impl AsRef<str>) -> GlobOutput {
    GlobOutput::from_uri(create_uri("glob", path)).unwrap()
}
