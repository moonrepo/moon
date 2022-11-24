use crate::depman::NodeDependencyManager;
use log::debug;
use probe_core::{
    async_trait, download_from_url, Describable, Downloadable, ProbeError, Resolvable,
};
use std::path::{Path, PathBuf};

#[async_trait]
impl Downloadable<'_> for NodeDependencyManager {
    fn get_download_path(&self) -> Result<PathBuf, ProbeError> {
        Ok(self
            .temp_dir
            .join(format!("{}.tgz", self.get_resolved_version())))
    }

    async fn download(&self, to_file: &Path, from_url: Option<&str>) -> Result<bool, ProbeError> {
        if to_file.exists() {
            debug!(target: self.get_log_target(), "Dependency manager already downloaded, continuing");

            return Ok(false);
        }

        let from_url = match from_url {
            Some(url) => url.to_owned(),
            None => {
                if self.dist.is_some() {
                    self.get_dist().tarball.clone()
                } else {
                    format!(
                        "https://registry.npmjs.org/npm/-/{}-{}.tgz",
                        self.type_of.get_package_name(),
                        self.get_resolved_version()
                    )
                }
            }
        };

        debug!(target: self.get_log_target(), "Attempting to download from {}", from_url);

        download_from_url(&from_url, &to_file).await?;

        debug!(target: self.get_log_target(), "Successfully downloaded dependency manager");

        Ok(true)
    }
}
