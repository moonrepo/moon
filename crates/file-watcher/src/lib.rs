use async_trait::async_trait;
use moon_common::path::WorkspaceRelativePathBuf;

#[derive(Clone, Debug)]
pub struct FileEvent {
    pub path: WorkspaceRelativePathBuf,
    pub kind: FileEventKind,
}

#[derive(Clone, Debug)]
pub enum FileEventKind {
    /// File or directory was created or modified
    Any,
    /// Continuous/ongoing modification (e.g. a long write)
    AnyContinuous,
}

#[async_trait]
pub trait FileWatcher<T> {
    async fn on_file_event(&self, event: FileEvent, state: T);
}
