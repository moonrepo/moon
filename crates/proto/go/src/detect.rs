use crate::GoLanguage;
use proto_core::{async_trait, load_version_file, Detector, ProtoError};
use std::path::Path;

#[async_trait]
impl Detector<'_> for GoLanguage {
    async fn detect_version_from(&self, working_dir: &Path) -> Result<Option<String>, ProtoError> {
        let gowork = working_dir.join("go.work");

        if gowork.exists() {
            return Ok(Some(scan_for_go_version(&gowork)?));
        }

        let gomod = working_dir.join("go.mod");

        if gomod.exists() {
            return Ok(Some(load_version_file(&gomod)?));
        }

        Ok(None)
    }
}

fn scan_for_go_version(path: &Path) -> Result<String, ProtoError> {
    // TODO
    Ok(String::from("1.19"))
}
