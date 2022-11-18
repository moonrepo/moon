use crate::errors::ProbeError;
use lenient_semver::Version;
use serde::de::DeserializeOwned;

#[async_trait::async_trait]
pub trait Resolvable<'tool, T: Send + Sync>: Send + Sync {
    /// Return the resolved version.
    fn get_resolved_version(&self) -> &str;

    /// Given an initial version, resolve it to a fully qualifed and semantic version
    /// according to the tool's ecosystem.
    async fn resolve_version(&self, initial_version: &str) -> Result<String, ProbeError>;
}

pub async fn load_versions_manifest<T: DeserializeOwned>(url: &str) -> Result<T, ProbeError> {
    let handle_error = |e: reqwest::Error| ProbeError::Http(url.to_owned(), e.to_string());

    let response = reqwest::get(url).await.map_err(handle_error)?;
    let content = response.text().await.map_err(handle_error)?;

    let manifest: T = serde_json::from_str(&content)
        .map_err(|e| ProbeError::Http(url.to_owned(), e.to_string()))?;

    Ok(manifest)
}

pub fn parse_version(version: &str) -> Result<Version, ProbeError> {
    Version::parse(version)
        .map_err(|e| ProbeError::VersionParseFailed(version.to_owned(), e.to_string()))
}
