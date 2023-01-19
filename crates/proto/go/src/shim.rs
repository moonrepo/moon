use crate::GoLanguage;
use proto_core::{
    async_trait, Executable, Installable, ProtoError, Resolvable, ShimBuilder, Shimable,
};
use std::path::Path;

#[async_trait]
impl Shimable<'_> for GoLanguage {
    async fn create_shims(&mut self) -> Result<(), ProtoError> {
        let install_path = self.get_install_dir()?;

        // Windows shims are poor at handling arguments, revisit
        if cfg!(not(windows)) {
            let shim_path = ShimBuilder::new("go", self.get_bin_path()?)
                .dir(install_path.join("go"))
                .version(self.get_resolved_version())
                .create()?;

            self.shim_path = Some(shim_path);
        }

        Ok(())
    }

    fn get_shim_path(&self) -> Option<&Path> {
        self.shim_path.as_deref()
    }
}
