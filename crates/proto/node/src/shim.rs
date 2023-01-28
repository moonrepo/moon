use crate::NodeLanguage;
use proto_core::{
    async_trait, Executable, Installable, ProtoError, Resolvable, ShimBuilder, Shimable,
};
use std::path::Path;

#[async_trait]
impl Shimable<'_> for NodeLanguage {
    async fn create_shims(&mut self) -> Result<(), ProtoError> {
        // Windows shims are poor at handling arguments, revisit
        if cfg!(not(windows)) {
            let shim_path = ShimBuilder::new("node", self.get_bin_path()?)
                .dir(self.get_install_dir()?)
                .version(self.get_resolved_version())
                .create_tool_shim()?;

            self.shim_path = Some(shim_path);
        }

        Ok(())
    }

    fn get_shim_path(&self) -> Option<&Path> {
        self.shim_path.as_deref()
    }
}
