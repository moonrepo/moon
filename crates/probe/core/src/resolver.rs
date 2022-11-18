use crate::errors::ProbeError;
use lenient_semver::Version;

#[async_trait::async_trait]
pub trait Resolver<T: Send + Sync>: Send + Sync {
    /// Given an initial version, resolve it to a fully qualifed and semantic version
    /// according to the tool's ecosystem.
    async fn resolve_version(&self, initial_version: &str) -> Result<Version, ProbeError>;
}
