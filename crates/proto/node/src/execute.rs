use crate::NodeLanguage;
use proto_core::{async_trait, Describable, Executable, Installable, ProtoError};
use std::path::Path;

#[cfg(target_os = "windows")]
pub fn get_bin_name<T: AsRef<str>>(name: T) -> String {
    format!("{}.{}", name.as_ref(), "exe")
}

#[cfg(not(target_os = "windows"))]
pub fn get_bin_name<T: AsRef<str>>(name: T) -> String {
    format!("bin/{}", name.as_ref())
}

#[async_trait]
impl Executable<'_> for NodeLanguage {
    async fn find_bin_path(&mut self) -> Result<(), ProtoError> {
        let dir = self.get_install_dir()?;
        let bin_path = dir.join(get_bin_name("node"));
        let shim_path = dir.join("node");

        if shim_path.exists() {
            self.shim_path = Some(shim_path);
        }

        if bin_path.exists() {
            self.bin_path = Some(bin_path);
        } else {
            return Err(ProtoError::ExecuteMissingBin(self.get_name(), bin_path));
        }

        Ok(())
    }

    fn get_bin_path(&self) -> Result<&Path, ProtoError> {
        if let Some(shim) = self.shim_path.as_ref() {
            return Ok(shim);
        }

        match self.bin_path.as_ref() {
            Some(bin) => Ok(bin),
            None => Err(ProtoError::MissingTool(self.get_name())),
        }
    }
}
