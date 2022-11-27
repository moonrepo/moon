use crate::errors::ProtoError;
use lenient_semver::Version;
use log::trace;
use serde::de::DeserializeOwned;

#[async_trait::async_trait]
pub trait Resolvable<'tool>: Send + Sync {
    /// Return the resolved version.
    fn get_resolved_version(&self) -> &str;

    /// Given an initial version, resolve it to a fully qualifed and semantic version
    /// according to the tool's ecosystem. A custom manifest URL can be provided as
    /// the 2nd argument.
    async fn resolve_version(
        &mut self,
        initial_version: &str,
        manifest_url: Option<&str>,
    ) -> Result<String, ProtoError>;
}

// Aliases are words that map to version. For example, "latest" -> "1.2.3".
pub fn is_version_alias(value: &str) -> bool {
    value.chars().all(|c| char::is_ascii_alphanumeric(&c))
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
