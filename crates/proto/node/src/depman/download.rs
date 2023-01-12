use crate::depman::NodeDependencyManager;
use log::debug;
use proto_core::{
    async_trait, color, download_from_url, Describable, Downloadable, ProtoError, Resolvable,
};
use std::path::{Path, PathBuf};

#[async_trait]
impl Downloadable<'_> for NodeDependencyManager {
    fn get_download_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self
            .temp_dir
            .join(format!("{}.tgz", self.get_resolved_version())))
    }

    async fn download(&self, to_file: &Path, from_url: Option<&str>) -> Result<bool, ProtoError> {
        if to_file.exists() {
            debug!(target: self.get_log_target(), "Dependency manager already downloaded, continuing");

            return Ok(false);
        }

        let pkg_name = &self.package_name;
        let from_url = match from_url {
            Some(url) => url.to_owned(),
            None => {
                format!(
                    "https://registry.npmjs.org/{}/-/{}-{}.tgz",
                    pkg_name,
                    pkg_name,
                    self.get_resolved_version()
                )
            }
        };

        debug!(target: self.get_log_target(), "Attempting to download from {}", color::url(&from_url));

        download_from_url(&from_url, &to_file).await?;

        debug!(target: self.get_log_target(), "Successfully downloaded dependency manager");

        Ok(true)
    }
}
