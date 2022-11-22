use crate::download::get_archive_file_path;
use crate::{depman::NodeDependencyManager, NodeDependencyManagerType};
use log::debug;
use probe_core::{async_trait, untar, unzip, Installable, ProbeError, Resolvable};
use std::path::{Path, PathBuf};

#[async_trait]
impl<'tool> Installable<'tool> for NodeDependencyManager<'tool> {
    fn get_install_dir(&self) -> Result<PathBuf, ProbeError> {
        Ok(self.install_dir.join(self.get_resolved_version()))
    }

    async fn install(&self, install_dir: &Path, download_path: &Path) -> Result<(), ProbeError> {
        if install_dir.exists() {
            debug!(target: "probe:node:install", "Already installed, continuing");

            return Ok(());
        }

        // This may not be accurate for all releases!
        let prefix = if matches!(&self.type_of, NodeDependencyManagerType::Yarn) {
            format!("yarn-v{}", self.get_resolved_version())
        } else {
            "package".into()
        };

        debug!(
            target: "probe:node:install",
            "Attempting to install from {} to {}",
            download_path.to_string_lossy(),
            install_dir.to_string_lossy(),
        );

        untar(download_path, install_dir, Some(&prefix))?;

        debug!(target: "probe:node:install", "Successfully installed");

        Ok(())
    }
}
