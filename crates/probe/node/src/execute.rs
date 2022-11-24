use crate::NodeLanguage;
use probe_core::{async_trait, Describable, Executable, Installable, ProbeError};
use std::path::PathBuf;

#[cfg(target_os = "windows")]
pub fn get_bin_name<T: AsRef<str>>(name: T) -> String {
    format!("{}.{}", name.as_ref(), "exe")
}

#[cfg(not(target_os = "windows"))]
pub fn get_bin_name_suffix<T: AsRef<str>>(name: T) -> String {
    format!("bin/{}", name.as_ref())
}

#[async_trait]
impl Executable<'_> for NodeLanguage {
    async fn find_bin_path(&mut self) -> Result<(), ProbeError> {
        let bin_path = self.get_install_dir()?.join(get_bin_name_suffix("node"));

        if bin_path.exists() {
            self.bin_path = Some(bin_path);
        } else {
            return Err(ProbeError::ExecuteMissingBin(self.get_name(), bin_path));
        }

        Ok(())
    }

    fn get_bin_path(&self) -> &PathBuf {
        self.bin_path.as_ref().expect("Missing Node.js bin path.")
    }
}
