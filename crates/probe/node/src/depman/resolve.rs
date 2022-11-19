#![allow(clippy::disallowed_types)]

use crate::depman::NodeDependencyManager;
use log::debug;
use probe_core::{
    async_trait, is_version_alias, load_versions_manifest, parse_version, remove_v_prefix,
    ProbeError, Resolvable,
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct NDMVersionDistSignature {
    keyid: String,
    sig: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NDMVersionDist {
    file_count: usize,
    integrity: String,
    shasum: String,
    #[serde(rename = "npm-signature")]
    signature: String,
    signatures: Vec<NDMVersionDistSignature>,
    tarball: String,
    unpacked_size: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NDMVersion {
    dist: NDMVersionDist,
    git_head: String,
    version: String, // No v prefix
}

#[derive(Deserialize)]
struct NDMManifest {
    #[serde(rename = "dist-tags")]
    dist_tags: HashMap<String, String>,
    versions: HashMap<String, NDMVersion>,
}

#[async_trait]
impl<'tool> Resolvable<'tool> for NodeDependencyManager<'tool> {
    fn get_resolved_version(&self) -> &str {
        &self.version
    }

    async fn resolve_version(
        &mut self,
        initial_version: &str,
        manifest_url: Option<&str>,
    ) -> Result<String, ProbeError> {
        let mut candidate = None;
        let mut initial_version = remove_v_prefix(initial_version);

        debug!(
            target: "probe:node-dep-man:resolve",
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
                Some(version) => version,
                None => {
                    return Err(ProbeError::VersionUnknownAlias(initial_version));
                }
            };
        }

        // Infer the possible candidate from the versions map
        candidate = match manifest.versions.get(&initial_version) {
            Some(version) => Some(&version.version),
            None => return Err(ProbeError::VersionResolveFailed(initial_version)),
        };

        let version = parse_version(candidate)?.to_string();

        debug!(target: "probe:node-depman:resolver", "Resolved to {}", version);

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
