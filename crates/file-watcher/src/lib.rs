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

    pub fn is_mutated_directory(&self) -> bool {
        matches!(
            self.kind,
            EventKind::Create(CreateKind::Folder)
                | EventKind::Modify(ModifyKind::Name(RenameMode::Both))
                | EventKind::Remove(RemoveKind::Folder)
        )
    }
}

#[async_trait]
pub trait FileWatcher<T>: Send + Sync {
    async fn on_file_event(&mut self, state: &mut T, event: &FileEvent) -> miette::Result<()>;
}

pub type BoxedFileWatcher<T> = Box<dyn FileWatcher<T>>;
