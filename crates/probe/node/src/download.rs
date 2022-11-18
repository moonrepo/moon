use crate::platform::NodeArch;
use crate::tool::NodeLanguage;
use probe_core::{async_trait, download_from_url, Downloadable, Probe, ProbeError, Resolvable};
use std::env::consts;
use std::path::PathBuf;
use std::str::FromStr;

#[cfg(target_os = "macos")]
pub fn get_archive_file_path(version: &str) -> Result<String, ProbeError> {
    let arch = NodeArch::from_str(consts::ARCH)?;

    if !matches!(NodeArch::X64 | NodeArch::Arm64) {
        return Err(ProbeError::UnsupportedArchitecture(
            "Node.js".into(),
            arch.to_string(),
        ));
    }

    Ok(format!("node-v{version}-darwin-{arch}"))
}

#[cfg(target_os = "linux")]
pub fn get_archive_file_path(version: &str) -> Result<String, ProbeError> {
    let arch = NodeArch::from_str(consts::ARCH)?;

    if !matches!(
        NodeArch::X64 | NodeArch::Arm64 | NodeArch::Armv7l | NodeArch::Ppc64le | NodeArch::S390x
    ) {
        return Err(ProbeError::UnsupportedArchitecture(
            "Node.js".into(),
            arch.to_string(),
        ));
    }

    Ok(format!("node-v{version}-linux-{arch}"))
}

#[cfg(target_os = "windows")]
pub fn get_archive_file_path(version: &str) -> Result<String, ProbeError> {
    let arch = NodeArch::from_str(consts::ARCH)?;

    if !matches!(NodeArch::X64 | NodeArch::X86) {
        return Err(ProbeError::UnsupportedArchitecture(
            "Node.js".into(),
            arch.to_string(),
        ));
    }

    Ok(format!("node-v{version}-win-{arch}"))
}

pub fn get_archive_file(version: &str) -> Result<String, ProbeError> {
    let ext = if consts::OS == "windows" {
        "zip"
    } else {
        "tar.gz"
    };

    Ok(format!("{}.{}", get_archive_file_path(version)?, ext))
}

#[async_trait]
impl<'tool> Downloadable<'tool, Probe> for NodeLanguage<'tool> {
    fn get_download_path(&self, parent: &Probe) -> Result<PathBuf, ProbeError> {
        Ok(parent
            .temp_dir
            .join(get_archive_file(self.get_resolved_version())?))
    }

    async fn is_downloaded(&self, parent: &Probe) -> Result<bool, ProbeError> {
        Ok(self.get_download_path(parent)?.exists())
    }

    async fn download(&self, parent: &Probe) -> Result<(), ProbeError> {
        let version = self.get_resolved_version();
        let download_file = self.get_download_path(parent)?;
        let download_url = format!(
            "https://nodejs.org/dist/v{}/{}",
            version,
            get_archive_file(version)?
        );

        download_from_url(&download_url, &download_file).await?;

        Ok(())
    }
}
