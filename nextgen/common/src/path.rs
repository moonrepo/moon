pub use relative_path::{RelativePath, RelativePathBuf};

// Named types for better readability
pub type ProjectRelativePath = RelativePath;
pub type ProjectRelativePathBuf = RelativePathBuf;
pub type WorkspaceRelativePath = RelativePath;
pub type WorkspaceRelativePathBuf = RelativePathBuf;

#[inline]
pub fn standardize_separators<T: AsRef<str>>(path: T) -> String {
    path.as_ref().replace('\\', "/")
}

pub enum RelativeFrom<'path> {
    Project(&'path str),
    Workspace,
}

pub fn expand_to_workspace_relative<P: AsRef<str>>(
    from_format: RelativeFrom,
    path: P,
) -> WorkspaceRelativePathBuf {
    let path = standardize_separators(path.as_ref());

    match from_format {
        RelativeFrom::Project(source) => {
            // Root-level project
            if source.is_empty() || source == "." {
                WorkspaceRelativePathBuf::from(path)

                // Project-level, prefix with source path
            } else {
                let project_source = standardize_separators(source);

                if let Some(negated_glob) = path.strip_prefix('!') {
                    WorkspaceRelativePathBuf::from(format!("!{project_source}")).join(negated_glob)
                } else {
                    WorkspaceRelativePathBuf::from(project_source).join(path)
                }
            }
        }
        RelativeFrom::Workspace => WorkspaceRelativePathBuf::from(path),
    }
}
