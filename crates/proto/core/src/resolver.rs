use crate::errors::ProtoError;
use lenient_semver::Version;
use log::trace;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;

pub struct VersionManifestEntry {
    pub alias: Option<String>,
    pub version: String,
}

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

// Aliases are words that map to version. For example, "latest" -> "1.2.3".
pub fn is_version_alias(value: &str) -> bool {
    value
        .chars()
        .all(|c| char::is_ascii_alphabetic(&c) || c == '-')
}

pub fn add_v_prefix(value: &str) -> String {
    if value.starts_with('v') || value.starts_with('V') {
        return value.to_lowercase();
    }

    format!("v{}", value)
}

pub fn remove_v_prefix(value: &str) -> String {
    if value.starts_with('v') || value.starts_with('V') {
        return value[1..].to_owned();
    }

    value.to_owned()
}

pub async fn load_versions_manifest<T, U>(url: U) -> Result<T, ProtoError>
where
    T: DeserializeOwned,
    U: AsRef<str>,
{
    let url = url.as_ref();
    let handle_error = |e: reqwest::Error| ProtoError::Http(url.to_owned(), e.to_string());

    trace!(
        target: "proto:resolver",
        "Loading versions manifest from {}",
        url
    );

    let response = reqwest::get(url).await.map_err(handle_error)?;
    let content = response.text().await.map_err(handle_error)?;

    let manifest: T = serde_json::from_str(&content)
        .map_err(|e| ProtoError::Http(url.to_owned(), e.to_string()))?;

    Ok(manifest)
}

pub fn parse_version(version: &str) -> Result<Version, ProtoError> {
    Version::parse(version)
        .map_err(|e| ProtoError::VersionParseFailed(version.to_owned(), e.to_string()))
}
