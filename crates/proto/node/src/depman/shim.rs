use crate::NodeDependencyManager;
use proto_core::{async_trait, ProtoError, Shimable};

#[async_trait]
impl Shimable<'_> for NodeDependencyManager {
    async fn create_shims(&self) -> Result<(), ProtoError> {
        Ok(())
    }
}
