use crate::download::get_archive_file_path;
use crate::NodeLanguage;
use proto_core::{async_trait, Installable, ProtoError, Resolvable};
use std::path::PathBuf;

#[async_trait]
impl Installable<'_> for NodeLanguage {
    fn get_archive_prefix(&self) -> Result<Option<String>, ProtoError> {
        Ok(Some(get_archive_file_path(self.get_resolved_version())?))
    }

    fn get_install_dir(&self) -> Result<PathBuf, ProtoError> {
        Ok(self.base_dir.join(self.get_resolved_version()))
    }
}
