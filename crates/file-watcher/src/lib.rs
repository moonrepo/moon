use async_trait::async_trait;
use moon_common::path::WorkspaceRelativePathBuf;
use std::sync::Arc;

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
pub trait FileWatcher<T>: Send + Sync {
    async fn on_file_event(&self, state: &mut T, event: &FileEvent) -> miette::Result<()>;
}

pub type BoxedFileWatcher<T> = Arc<dyn FileWatcher<T>>;
