use crate::GoLanguage;
use crate::platform::GoArch;
use moon_utils::semver::Version;
use proto_core::{async_trait, Downloadable, ProtoError, Resolvable};
use std::env::consts;
use std::path::PathBuf;

#[cfg(target_os = "macos")]
pub fn get_archive_file_path(version: &str) -> Result<String, ProtoError> {
    let arch = GoArch::from_os_arch()?;

    if !matches!(arch, GoArch::Amd64 | GoArch::Arm64) {
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
        GoArch::I386 | GoArch::Amd64 | GoArch::Arm64 | GoArch::Armv6l | GoArch::S390x
    ) {
        return Err(ProtoError::UnsupportedArchitecture(
            "Go".into(),
            arch.to_string(),
        ));
    }

    Ok(format!("go{version}.linux-{arch}"))
}

#[cfg(target_os = "windows")]
pub fn get_archive_file_path(version: &str) -> Result<String, ProtoError> {
    let arch = GoArch::from_os_arch()?;

    if !matches!(arch, GoArch::I386 | GoArch::Amd64 | GoArch::Arm64) {
        return Err(ProtoError::UnsupportedArchitecture(
            "Go".into(),
            arch.to_string(),
        ));
    }

    Ok(format!("go{version}.windows-{arch}"))
}

pub fn get_archive_file(version: &str) -> Result<String, ProtoError> {
    let ext = if consts::OS == "windows" {
        "zip"
    } else {
        "tar.gz"
    };

    let v = match Version::parse(version) {
        Ok(a) => a,
        Err(e) => {
            return Err(ProtoError::DownloadFailed(version.into(), e.to_string()))
        }
    };

    if v.patch == 0 {
        let short_version = format!("{}.{}", v.major, v.minor);
        return Ok(format!("{}.{}", get_archive_file_path(short_version.as_str())?, ext));
    }

    Ok(format!("{}.{}", get_archive_file_path(version)?, ext))
}

#[async_trait]
impl Downloadable<'_> for GoLanguage {
    fn get_download_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self
            .temp_dir
            .join(get_archive_file(self.get_resolved_version())?))
    }

    fn get_download_url(&self) -> Result<String, ProtoError> {
        Ok(format!(
            "https://go.dev/dl/{}",
            get_archive_file(self.get_resolved_version())?
        ))
    }
}
