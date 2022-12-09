use crate::NodeLanguage;
use proto_core::{async_trait, load_version_file, Detector, ProtoError};
use std::path::Path;

#[async_trait]
impl Detector<'_> for NodeLanguage {
    async fn detect_version_from(&self, working_dir: &Path) -> Result<Option<String>, ProtoError> {
        let nvmrc = working_dir.join(".nvmrc");

        if nvmrc.exists() {
            return Ok(Some(load_version_file(&nvmrc)?));
        }

        let nodenv = working_dir.join(".node-version");

        if nodenv.exists() {
            return Ok(Some(load_version_file(&nodenv)?));
        }

        Ok(None)
    }
}
