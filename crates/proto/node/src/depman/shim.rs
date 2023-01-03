use crate::NodeDependencyManager;
use proto_core::{
    async_trait, Executable, Installable, ProtoError, Resolvable, ShimBuilder, Shimable,
};
use std::path::Path;

#[async_trait]
impl Shimable<'_> for NodeDependencyManager {
    async fn create_shims(&self) -> Result<(), ProtoError> {
        ShimBuilder::new(&self.type_of.get_package_name(), self.get_bin_path()?)
            .dir(self.get_install_dir()?)
            .version(self.get_resolved_version())
            .parent("node")
            .create()?;

        Ok(())
    }

    fn get_shim_path(&self) -> Option<&Path> {
        match self.shim_path.as_ref() {
            Some(path) => Some(path),
            None => None,
        }
    }
}
