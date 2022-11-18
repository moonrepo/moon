use crate::errors::ProbeError;

#[async_trait::async_trait]
pub trait Resolvable<'tool, T: Send + Sync>: Send + Sync {
    /// Return the resolved version.
    fn get_resolved_version(&self) -> &str;

    /// Given an initial version, resolve it to a fully qualifed and semantic version
    /// according to the tool's ecosystem.
    async fn resolve_version(&self, initial_version: &str) -> Result<String, ProbeError>;
}
