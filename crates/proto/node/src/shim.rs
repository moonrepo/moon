use crate::NodeLanguage;
use proto_core::{async_trait, Executable, ProtoError, ShimBuilder, Shimable};

#[async_trait]
impl Shimable<'_> for NodeLanguage {
    async fn create_shims(&self) -> Result<(), ProtoError> {
        let mut builder = ShimBuilder::new("node", self.get_bin_path()?)
            .install_dir(&self.install_dir)
            .version(&self.version);

        Ok(())
    }
}
