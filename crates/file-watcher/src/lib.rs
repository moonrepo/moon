use async_trait::async_trait;
use moon_common::path::WorkspaceRelativePathBuf;

pub use notify_types::event::*;

#[derive(Clone, Debug)]
pub struct FileEvent {
    pub path: WorkspaceRelativePathBuf,
    pub kind: EventKind,
}

#[async_trait]
pub trait FileWatcher<T>: Send + Sync {
    async fn on_file_event(&mut self, state: &mut T, event: &FileEvent) -> miette::Result<()>;
}

pub type BoxedFileWatcher<T> = Box<dyn FileWatcher<T>>;
