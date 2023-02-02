use crate::{depman::NodeDependencyManager, NodeDependencyManagerType};
use proto_core::{async_trait, Installable, ProtoError, Resolvable};
use std::path::PathBuf;

#[async_trait]
impl Installable<'_> for NodeDependencyManager {
    fn get_archive_prefix(&self) -> Result<Option<String>, ProtoError> {
        Ok(Some(
            if matches!(&self.type_of, NodeDependencyManagerType::Yarn) {
                format!("yarn-v{}", self.get_resolved_version())
            } else {
                "package".into()
            },
        ))
    }

    fn get_install_dir(&self) -> Result<PathBuf, ProtoError> {
        Ok(self.base_dir.join(self.get_resolved_version()))
    }
}
