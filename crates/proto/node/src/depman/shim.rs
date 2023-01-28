use crate::NodeDependencyManager;
use proto_core::{
    async_trait, Executable, Installable, ProtoError, Resolvable, ShimBuilder, Shimable,
};
use std::path::Path;

#[async_trait]
impl Shimable<'_> for NodeDependencyManager {
    async fn create_shims(&mut self) -> Result<(), ProtoError> {
        // Windows shims are poor at handling arguments, revisit
        if cfg!(not(windows)) {
            let mut shimmer = ShimBuilder::new(&self.package_name, self.get_bin_path()?);

            shimmer
                .dir(self.get_install_dir()?)
                .version(self.get_resolved_version())
                .parent("node");

            shimmer.create_global_shim()?;

            self.shim_path = Some(shimmer.create_tool_shim()?);
        }

        Ok(())
    }

    fn get_shim_path(&self) -> Option<&Path> {
        self.shim_path.as_deref()
    }
}
