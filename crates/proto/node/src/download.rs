use crate::platform::NodeArch;
use crate::NodeLanguage;
use proto_core::{async_trait, Downloadable, ProtoError, Resolvable};
use std::env::consts;
use std::path::PathBuf;

#[cfg(target_os = "macos")]
pub fn get_archive_file_path(version: &str) -> Result<String, ProtoError> {
    let arch = NodeArch::from_os_arch()?;

    if !matches!(arch, NodeArch::X64 | NodeArch::Arm64) {
        return Err(ProtoError::UnsupportedArchitecture(
            "Node.js".into(),
            arch.to_string(),
        ));
    }

    Ok(format!("node-v{version}-darwin-{arch}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn get_archive_file_path(version: &str) -> Result<String, ProtoError> {
    let arch = NodeArch::from_os_arch()?;

    if !matches!(
        arch,
        NodeArch::X64 | NodeArch::Arm | NodeArch::Arm64 | NodeArch::Ppc64 | NodeArch::S390x
    ) {
        return Err(ProtoError::UnsupportedArchitecture(
            "Node.js".into(),
            arch.to_string(),
        ));
    }

    Ok(format!("node-v{version}-linux-{arch}"))
}

#[cfg(target_os = "windows")]
pub fn get_archive_file_path(version: &str) -> Result<String, ProtoError> {
    let arch = NodeArch::from_os_arch()?;

    if !matches!(arch, NodeArch::X64 | NodeArch::X86) {
        return Err(ProtoError::UnsupportedArchitecture(
            "Node.js".into(),
            arch.to_string(),
        ));
    }

    Ok(format!("node-v{version}-win-{arch}"))
}

pub fn get_archive_file(version: &str) -> Result<String, ProtoError> {
    let ext = if consts::OS == "windows" {
        "zip"
    } else {
        "tar.gz"
    };

    Ok(format!("{}.{}", get_archive_file_path(version)?, ext))
}

#[async_trait]
impl Downloadable<'_> for NodeLanguage {
    fn get_download_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self
            .temp_dir
            .join(get_archive_file(self.get_resolved_version())?))
    }

    fn get_download_url(&self) -> Result<String, ProtoError> {
        let version = self.get_resolved_version();

        Ok(format!(
            "https://nodejs.org/dist/v{}/{}",
            version,
            get_archive_file(version)?
        ))
    }
}
