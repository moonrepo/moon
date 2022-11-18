use crate::tool::NodeLanguage;
use probe_core::{async_trait, Probe, ProbeError, Resolvable, Version};

#[async_trait]
impl<'tool> Resolvable<'tool, Probe> for NodeLanguage<'tool> {
    fn get_resolved_version(&self) -> &str {
        &self.version
    }

    async fn resolve_version(&self, initial_version: &str) -> Result<String, ProbeError> {
        let version = Version::parse(initial_version).map_err(|e| {
            ProbeError::VersionParseFailed(initial_version.to_owned(), e.to_string())
        })?;

        Ok(version.to_string())
    }
}
