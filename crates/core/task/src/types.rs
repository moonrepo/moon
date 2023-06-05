use moon_common::path::WorkspaceRelativePathBuf;
use rustc_hash::FxHashSet;

pub type TouchedFilePaths = FxHashSet<WorkspaceRelativePathBuf>;
