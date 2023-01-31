use crate::GoLanguage;
use log::error;
use proto_core::{async_trait, Detector, ProtoError};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

static GOPREFIX: &str = "go ";

#[async_trait]
impl Detector<'_> for GoLanguage {
    async fn detect_version_from(&self, working_dir: &Path) -> Result<Option<String>, ProtoError> {
        let gowork = working_dir.join("go.work");

        if gowork.exists() {
            if let Some(version) = scan_for_go_version(&gowork) {
                return Ok(Some(version));
            }
        }

        let gomod = working_dir.join("go.mod");

        if gomod.exists() {
            if let Some(version) = scan_for_go_version(&gomod) {
                return Ok(Some(version));
            }
        }

        Ok(None)
    }
}

fn scan_for_go_version(path: &Path) -> Option<String> {
    match File::open(path) {
        Ok(file) => {
            let buffered = BufReader::new(file);
            for line in buffered.lines().flatten() {
                if let Some(version) = line.strip_prefix(GOPREFIX) {
                    return Some(version.into());
                }
            }
        }
        Err(e) => {
            error!("{} failed to load {}", path.to_str().unwrap(), e);
            return None;
        }
    }

    error!("no go version found in {}", path.to_str().unwrap());

    None
}
