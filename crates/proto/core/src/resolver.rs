use crate::color;
use crate::{get_temp_dir, is_version_alias, remove_v_prefix};
use lenient_semver::Version;
use log::trace;
use proto_error::ProtoError;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};
use std::{fs, io};

#[derive(Debug)]
pub struct VersionManifestEntry {
    pub alias: Option<String>,
    pub version: String,
}

#[derive(Debug)]
pub struct VersionManifest {
    pub aliases: BTreeMap<String, String>,
    pub versions: BTreeMap<String, VersionManifestEntry>,
}

impl VersionManifest {
    pub fn find_version(&self, version: &str) -> Result<&String, ProtoError> {
        if is_version_alias(version) {
            return self.find_version_from_alias(version);
        }

        let prefixless_version = remove_v_prefix(version);

        // Matching against explicit version
        if let Some(entry) = self.versions.get(&prefixless_version) {
            return Ok(&entry.version);
        }

        // Match against a partial minor/patch range, for example, "10" -> "10.1.2".
        // We also parse versions instead of using starts with, as we need to ensure
        // "10.1" matches "10.1.*" and not "10.10.*"!
        let find_version = parse_version(version)?;

        for entry in self.versions.values().rev() {
            let entry_version = parse_version(&entry.version)?;

            if entry_version.major != find_version.major {
                continue;
            }

            if find_version.minor != 0 && entry_version.minor != find_version.minor {
                continue;
            }

            if find_version.patch != 0 && entry_version.patch != find_version.patch {
                continue;
            }

            return Ok(&entry.version);
        }

        Err(ProtoError::VersionResolveFailed(version.to_owned()))
    }

    pub fn find_version_from_alias(&self, alias: &str) -> Result<&String, ProtoError> {
        self.aliases
            .get(alias)
            .ok_or_else(|| ProtoError::VersionUnknownAlias(alias.to_owned()))
    }

    pub fn get_version(&self, version: &str) -> Result<&String, ProtoError> {
        if let Some(entry) = self.versions.get(version) {
            return Ok(&entry.version);
        }

        Err(ProtoError::VersionResolveFailed(version.to_owned()))
    }
}

#[async_trait::async_trait]
pub trait Resolvable<'tool>: Send + Sync {
    /// Return the resolved version.
    fn get_resolved_version(&self) -> &str;

    /// Load the upstream version and release manifest.
    async fn load_manifest(&self) -> Result<VersionManifest, ProtoError>;

    /// Given an initial version, resolve it to a fully qualifed and semantic version
    /// according to the tool's ecosystem.
    async fn resolve_version(&mut self, initial_version: &str) -> Result<String, ProtoError>;
}

pub async fn load_versions_manifest<T, U>(url: U) -> Result<T, ProtoError>
where
    T: DeserializeOwned,
    U: AsRef<str>,
{
    let url = url.as_ref();
    let mut sha = Sha256::new();
    sha.update(url);

    let temp_dir = get_temp_dir()?;
    let temp_file = temp_dir.join(format!("{:x}.json", sha.finalize()));
    let handle_http_error = |e: reqwest::Error| ProtoError::Http(url.to_owned(), e.to_string());
    let handle_io_error = |e: io::Error| ProtoError::Fs(temp_file.to_path_buf(), e.to_string());

    // If the resource has been cached within the last 24 hours, use it
    if temp_file.exists() {
        let metadata = fs::metadata(&temp_file).map_err(handle_io_error)?;

        if let Ok(modified_time) = metadata.modified().or_else(|_| metadata.created()) {
            let threshold = SystemTime::now() - Duration::from_secs(60 * 60 * 24);

            if modified_time > threshold {
                trace!(
                    target: "proto:resolver",
                    "Loading versions manifest from locally cached {}",
                    color::path(&temp_file),
                );

                let contents = fs::read_to_string(&temp_file).map_err(handle_io_error)?;

                return serde_json::from_str(&contents)
                    .map_err(|e| ProtoError::Fs(temp_file.to_path_buf(), e.to_string()));
            }
        }
    }

    // Otherwise, request the resource and cache it
    trace!(
        target: "proto:resolver",
        "Loading versions manifest from {}",
        color::url(url),
    );

    let response = reqwest::get(url).await.map_err(handle_http_error)?;
    let contents = response.text().await.map_err(handle_http_error)?;

    fs::create_dir_all(&temp_dir).map_err(handle_io_error)?;
    fs::write(&temp_file, &contents).map_err(handle_io_error)?;

    serde_json::from_str(&contents).map_err(|e| ProtoError::Http(url.to_owned(), e.to_string()))
}

pub fn parse_version(version: &str) -> Result<Version, ProtoError> {
    Version::parse(version)
        .map_err(|e| ProtoError::VersionParseFailed(version.to_owned(), e.to_string()))
}
