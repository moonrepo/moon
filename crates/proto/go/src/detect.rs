use crate::GoLanguage;
use proto_core::{async_trait, load_version_file, Detector, ProtoError};
use std::path::Path;
use std::fs;

#[async_trait]
impl Detector<'_> for GoLanguage {
    async fn detect_version_from(&self, working_dir: &Path) -> Result<Option<String>, ProtoError> {
        let gowork = working_dir.join("go.work");

        if gowork.exists() {
            return Ok(Some(scan_for_go_version(&gowork)?));
        }

        let gomod = working_dir.join("go.mod");

        if gomod.exists() {
            return Ok(Some(scan_for_go_version(&gomod)?));
        }

        Ok(None)
    }
}

fn scan_for_go_version(path: &Path) -> Result<String, ProtoError> {
    for line in fs::read_to_string(path).iter() {
        dbg!(&line);
        if line.starts_with("go ") {
            match line.strip_prefix("go ") {
                Some(version) => { return Ok(String::from(version)) },
                None => ()
            }
        }
    }

    // TODO
    Err(ProtoError::Fs(path.to_path_buf(), String::from("no go version found")))
}
