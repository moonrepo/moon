use crate::depman::NodeDependencyManager;
use proto_core::{async_trait, Downloadable, ProtoError, Resolvable};
use std::path::PathBuf;

#[async_trait]
impl Downloadable<'_> for NodeDependencyManager {
    fn get_download_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self
            .temp_dir
            .join(format!("{}.tgz", self.get_resolved_version())))
    }

    fn get_download_url(&self) -> Result<String, ProtoError> {
        let pkg_name = &self.package_name;

        Ok(format!(
            "https://registry.npmjs.org/{}/-/{}-{}.tgz",
            pkg_name,
            pkg_name,
            self.get_resolved_version()
        ))
    }
}
