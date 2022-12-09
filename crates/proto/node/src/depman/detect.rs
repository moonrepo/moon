use crate::depman::NodeDependencyManager;
use proto_core::{async_trait, load_version_file, Detector, ProtoError};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct PackageJson {
    #[serde(rename = "packageManager")]
    package_manager: Option<String>,
}

// https://nodejs.org/api/packages.html#packagemanager
#[async_trait]
impl Detector<'_> for NodeDependencyManager {
    async fn detect_version_from(&self, working_dir: &Path) -> Result<Option<String>, ProtoError> {
        let package_path = working_dir.join("package.json");

        if package_path.exists() {
            let package_json: PackageJson =
                serde_json::from_str(&load_version_file(&package_path)?)
                    .map_err(|e| ProtoError::Json(package_path.to_path_buf(), e.to_string()))?;

            if let Some(manager) = package_json.package_manager {
                let mut parts = manager.split('@');
                let name = parts.next().unwrap_or_default();

                if name == self.type_of.get_package_name() {
                    return Ok(Some(parts.next().unwrap_or("latest").to_owned()));
                }
            }
        }

        Ok(None)
    }
}
