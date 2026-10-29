use async_trait::async_trait;
use moon_common::path::WorkspaceRelativePathBuf;
use std::path::PathBuf;

pub use notify_types::event::*;

#[derive(Clone, Debug)]
pub struct FileEvent {
    pub path_original: PathBuf,
    pub path: WorkspaceRelativePathBuf,
    pub kind: EventKind,
}

impl FileEvent {
    pub fn is_mutated(&self) -> bool {
        self.kind.is_modify() || self.kind.is_create() || self.kind.is_remove()
    }

    /// Renames are included, as backends don't report whether a renamed
    /// path is a directory, and only some pair the old and new paths.
    pub fn is_mutated_directory(&self) -> bool {
        matches!(
            self.kind,
            EventKind::Create(CreateKind::Folder)
                | EventKind::Modify(ModifyKind::Name(_))
                | EventKind::Remove(RemoveKind::Folder)
        )
    }
}

#[async_trait]
pub trait FileWatcher<T>: Send + Sync {
    async fn on_init(&mut self, state: T) -> miette::Result<()>;
    async fn on_file_event(&mut self, state: T, event: &FileEvent) -> miette::Result<()>;
}

pub type BoxedFileWatcher<T> = Box<dyn FileWatcher<T>>;
