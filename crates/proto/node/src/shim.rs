use crate::NodeLanguage;
use proto_core::{
    async_trait, Executable, Installable, ProtoError, Resolvable, ShimBuilder, Shimable,
};

#[async_trait]
impl Shimable<'_> for NodeLanguage {
    async fn create_shims(&self) -> Result<(), ProtoError> {
        ShimBuilder::new("node", self.get_bin_path()?)
            .dir(self.get_install_dir()?)
            .version(self.get_resolved_version())
            .create()?;

        Ok(())
    }
}
