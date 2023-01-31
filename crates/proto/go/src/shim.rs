use crate::GoLanguage;
use proto_core::{
    async_trait, Executable, Installable, ProtoError, Resolvable, ShimBuilder, Shimable,
};
use std::path::Path;

#[async_trait]
impl Shimable<'_> for GoLanguage {
    async fn create_shims(&mut self) -> Result<(), ProtoError> {
        // Windows shims are poor at handling arguments, revisit
        if cfg!(not(windows)) {
            let mut shimmer = ShimBuilder::new("go", self.get_bin_path()?);

            let install_path = self.get_install_dir()?;

            shimmer
                .dir(install_path.join("go"))
                .version(self.get_resolved_version());

            shimmer.create_global_shim()?;

            self.shim_path = Some(shimmer.create_tool_shim()?);
        }

        Ok(())
    }

    fn get_shim_path(&self) -> Option<&Path> {
        self.shim_path.as_deref()
    }
}
