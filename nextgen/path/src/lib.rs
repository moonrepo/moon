pub use relative_path::{RelativePath, RelativePathBuf};

// pub enum PathType {
//     File(String),
//     Glob(String),
// }

// pub enum Location<T> {
//     Absolute(T),
//     ProjectRelative(T),
//     WorkspaceRelative(T),
// }

// Named types for better readability
pub type ProjectRelativePath = RelativePath;
pub type ProjectRelativePathBuf = RelativePathBuf;
pub type WorkspaceRelativePath = RelativePath;
pub type WorkspaceRelativePathBuf = RelativePathBuf;

#[inline]
pub fn standardize_separators<T: AsRef<str>>(path: T) -> String {
    path.as_ref().replace('\\', "/")
}

#[inline]
pub fn expand_to_workspace_relative<F, P>(file: F, project_source: P) -> WorkspaceRelativePathBuf
where
    F: AsRef<str>,
    P: AsRef<str>,
{
    let file = file.as_ref();
    let project_source = project_source.as_ref();

    // Workspace relative negative glob
    if file.starts_with("/!") || file.starts_with("!/") {
        return WorkspaceRelativePathBuf::from(format!("!{}", standardize_separators(&file[2..])))
            .normalize();
    }

    // Workspace relative file/glob
    if let Some(path) = file.strip_prefix('/') {
        return WorkspaceRelativePathBuf::from(standardize_separators(path)).normalize();
    }

    // Project relative negative glob
    if let Some(glob) = file.strip_prefix('!') {
        return WorkspaceRelativePathBuf::from(format!("!{}", project_source))
            .join(standardize_separators(glob))
            .normalize();
    }

    // Project relative file/glob
    WorkspaceRelativePathBuf::from(project_source)
        .join(standardize_separators(file))
        .normalize()
}
