use crate::platform::GoArch;
use crate::GoLanguage;
use log::debug;
use proto_core::{
    async_trait, color, download_from_url, Describable, Downloadable, ProtoError, Resolvable,
};
use std::env::consts;
use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
pub fn get_archive_file_path(version: &str) -> Result<String, ProtoError> {
    let arch = GoArch::from_os_arch()?;

    if !matches!(arch, GoArch::X64 | GoArch::Arm64) {
        return Err(ProtoError::UnsupportedArchitecture(
            "Go".into(),
            arch.to_string(),
        ));
    }

    Ok(format!("go{version}.darwin-{arch}"))
}

#[cfg(target_os = "linux")]
pub fn get_archive_file_path(version: &str) -> Result<String, ProtoError> {
    let arch = GoArch::from_os_arch()?;

    if !matches!(
        arch,
        GoArch::X64 | GoArch::Arm | GoArch::Arm64 | GoArch::Ppc64 | GoArch::S390x
    ) {
        return Err(ProtoError::UnsupportedArchitecture(
            "Go".into(),
            arch.to_string(),
        ));
    }

    // TODO update
    Ok(format!("go{version}.darwin-{arch}"))
}

#[cfg(target_os = "windows")]
pub fn get_archive_file_path(version: &str) -> Result<String, ProtoError> {
    let arch = GoArch::from_os_arch()?;

    if !matches!(arch, GoArch::X64 | GoArch::X86) {
        return Err(ProtoError::UnsupportedArchitecture(
            "Go".into(),
            arch.to_string(),
        ));
    }

    // TODO update
    Ok(format!("go{version}.darwin-{arch}"))
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
impl Downloadable<'_> for GoLanguage {
    fn get_download_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self
            .temp_dir
            .join(get_archive_file(self.get_resolved_version())?))
    }

    async fn download(&self, to_file: &Path, from_url: Option<&str>) -> Result<bool, ProtoError> {
        if to_file.exists() {
            debug!(target: self.get_log_target(), "Tool already downloaded, continuing");

            return Ok(false);
        }

        let version = self.get_resolved_version();
        let from_url = match from_url {
            Some(url) => url.to_owned(),
            None => {
                format!(
                    "https://dl.google.com/go/{}",
                    get_archive_file(version)?
                )
            }
        };

        debug!(target: self.get_log_target(), "Attempting to download tool from {}", color::url(&from_url));

        download_from_url(&from_url, &to_file).await?;

        debug!(target: self.get_log_target(), "Successfully downloaded tool");

        Ok(true)
    }
}
