use crate::depman::NodeDependencyManager;
use clean_path::Clean;
use proto_core::{async_trait, Describable, Executable, Installable, ProbeError};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub fn extract_bin_from_package_json(
    package_path: PathBuf,
    bin_name: &str,
) -> Result<Option<String>, ProbeError> {
    let mut bin_path = None;

    let data = fs::read_to_string(&package_path)
        .map_err(|e| ProbeError::Fs(package_path.clone(), e.to_string()))?;

    let json: Value =
        serde_json::from_str(&data).map_err(|e| ProbeError::Json(package_path, e.to_string()))?;

    if let Some(bin_field) = json.get("bin") {
        match bin_field {
            Value::String(bin) => {
                bin_path = Some(bin.to_owned());
            }
            Value::Object(bins) => {
                if let Some(bin) = bins.get(bin_name) {
                    bin_path = Some(bin.as_str().unwrap_or_default().to_string());
                }
            }
            _ => {}
        };
    }

    if bin_path.is_none() {
        if let Some(main_field) = json.get("main") {
            bin_path = Some(main_field.as_str().unwrap_or_default().to_string());
        }
    }

    Ok(bin_path)
}

#[async_trait]
impl Executable<'_> for NodeDependencyManager {
    async fn find_bin_path(&mut self) -> Result<(), ProbeError> {
        let install_dir = self.get_install_dir()?;
        let bin_name = self.type_of.get_package_name();
        let package_json = install_dir.join("package.json");

        if package_json.exists() {
            if let Some(bin_path) = extract_bin_from_package_json(package_json, &bin_name)? {
                self.bin_path = Some(install_dir.join(bin_path).clean());

                return Ok(());
            }
        }

        return Err(ProbeError::ExecuteMissingBin(
            self.get_name(),
            install_dir.join(format!("bin/{}.js", bin_name)),
        ));
    }

    fn get_bin_path(&self) -> Result<&Path, ProbeError> {
        match self.bin_path.as_ref() {
            Some(bin) => Ok(bin),
            None => Err(ProbeError::MissingTool(self.get_name())),
        }
    }
}
