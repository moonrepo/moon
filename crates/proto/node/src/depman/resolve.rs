use crate::depman::{NodeDependencyManager, NodeDependencyManagerType};
use log::debug;
use proto_core::{
    async_trait, is_version_alias, load_versions_manifest, parse_version, remove_v_prefix,
    Describable, ProtoError, Resolvable,
};
use rustc_hash::FxHashMap;
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct NDMVersionDistSignature {
    pub keyid: String,
    pub sig: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NDMVersionDist {
    pub integrity: String,
    pub shasum: String,
    #[serde(rename = "npm-signature")]
    pub signature: Option<String>,
    pub signatures: Vec<NDMVersionDistSignature>,
    pub tarball: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NDMVersion {
    dist: NDMVersionDist,
    version: String, // No v prefix
}

#[derive(Deserialize)]
struct NDMManifest {
    #[serde(rename = "dist-tags")]
    dist_tags: FxHashMap<String, String>,
    versions: FxHashMap<String, NDMVersion>,
}

#[async_trait]
impl Resolvable<'_> for NodeDependencyManager {
    fn get_resolved_version(&self) -> &str {
        &self.version
    }

    async fn resolve_version(
        &mut self,
        initial_version: &str,
        manifest_url: Option<&str>,
    ) -> Result<String, ProtoError> {
        let mut initial_version = remove_v_prefix(initial_version);

        // Yarn is installed through npm, but only v1 exists in the npm registry,
        // even if a consumer is using Yarn 2/3. https://www.npmjs.com/package/yarn
        // Yarn >= 2 work differently than normal packages, as their runtime code
        // is stored *within* the repository, and the v1 package detects it.
        // Because of this, we need to always install the v1 package!
        if matches!(&self.type_of, NodeDependencyManagerType::Yarn)
            && !initial_version.starts_with('1')
        {
            debug!(
                target: self.get_log_target(),
                "Found Yarn v2+, installing latest v1 from registry for compatibility"
            );

            initial_version = "latest".to_owned();
        }

        debug!(
            target: self.get_log_target(),
            "Resolving a semantic version for {}",
            initial_version,
        );

        let manifest_url = match manifest_url {
            Some(url) => url.to_owned(),
            None => format!(
                "https://registry.npmjs.org/{}/",
                self.type_of.get_package_name()
            ),
        };
        let manifest: NDMManifest = load_versions_manifest(manifest_url).await?;

        // Aliases map to dist tags
        if is_version_alias(&initial_version) {
            initial_version = match manifest.dist_tags.get(&initial_version) {
                Some(version) => version.to_owned(),
                None => {
                    return Err(ProtoError::VersionUnknownAlias(initial_version));
                }
            };
        }

        // Infer the possible candidate from the versions map
        let candidate = match manifest.versions.get(&initial_version) {
            Some(version) => Some(&version.version),
            None => return Err(ProtoError::VersionResolveFailed(initial_version)),
        };

        let version = parse_version(candidate.unwrap())?.to_string();

        debug!(target: self.get_log_target(), "Resolved to {}", version);

        self.version = version.clone();

        // Extract dist information for use in downloading and verifying
        self.dist = Some(
            manifest
                .versions
                .get(candidate.unwrap())
                .unwrap()
                .dist
                .clone(),
        );

        Ok(version)
    }
}
