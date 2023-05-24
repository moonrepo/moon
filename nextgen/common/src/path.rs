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
