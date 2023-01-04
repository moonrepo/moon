use crate::NodeDependencyManager;
use log::debug;
use proto_core::{
    async_trait, Describable, Executable, Installable, ProtoError, Resolvable, ShimBuilder,
    Shimable,
};
use std::path::Path;

#[async_trait]
impl Shimable<'_> for NodeDependencyManager {
    async fn create_shims(&mut self) -> Result<(), ProtoError> {
        let shim_path = ShimBuilder::new(&self.package_name, self.get_bin_path()?)
            .dir(self.get_install_dir()?)
            .version(self.get_resolved_version())
            .parent("node")
            .create()?;

        debug!(target: self.get_log_target(), "Created shim at {}", shim_path.to_string_lossy());

        self.shim_path = Some(shim_path);

        Ok(())
    }

    fn get_shim_path(&self) -> Option<&Path> {
        match self.shim_path.as_ref() {
            Some(path) => Some(path),
            None => None,
        }
    }
}
