use crate::tool::NodeLanguage;
use probe_core::{async_trait, download_from_url, Downloadable, Probe, ProbeError, Resolvable};
use std::env::consts;
use std::path::PathBuf;

pub fn get_archive_file_path(version: &str) -> Result<String, ProbeError> {
    let platform;

    if consts::OS == "linux" {
        platform = "linux"
    } else if consts::OS == "windows" {
        platform = "win";
    } else if consts::OS == "macos" {
        platform = "darwin"
    } else {
        return Err(ProbeError::UnsupportedPlatform(
            "Node.js".into(),
            consts::OS.to_string(),
        ));
    }

    let arch;

    if consts::ARCH == "x86" {
        arch = "x86"
    } else if consts::ARCH == "x86_64" {
        arch = "x64"
    } else if consts::ARCH == "arm" || consts::ARCH == "aarch64" {
        arch = "arm64"
    // } else if consts::ARCH == "powerpc64" {
    //     arch = "ppc64le"
    // } else if consts::ARCH == "s390x" {
    //     arch = "s390x"
    } else {
        return Err(ProbeError::UnsupportedArchitecture(
            "Node.js".into(),
            consts::ARCH.to_string(),
        ));
    }

    Ok(format!("node-v{version}-{platform}-{arch}"))
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
