#![allow(clippy::disallowed_types)]

use crate::NodeLanguage;
use log::debug;
use proto_core::{
    add_v_prefix, async_trait, is_version_alias, load_versions_manifest, parse_version,
    Describable, ProtoError, Resolvable,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(untagged)]
enum NodeLTS {
    Name(String),
    State(bool),
}

#[derive(Deserialize)]
struct NodeDistVersion {
    lts: NodeLTS,
    version: String, // Starts with v
}

#[async_trait]
impl Resolvable<'_> for NodeLanguage {
    fn get_resolved_version(&self) -> &str {
        &self.version
    }

    async fn resolve_version(
        &mut self,
        initial_version: &str,
        manifest_url: Option<&str>,
    ) -> Result<String, ProtoError> {
        let mut candidate = None;
        let initial_version = initial_version.to_lowercase();

        debug!(
            target: self.get_log_target(),
            "Resolving a semantic version for {}",
            initial_version,
        );

        let manifest: Vec<NodeDistVersion> =
            load_versions_manifest(manifest_url.unwrap_or("https://nodejs.org/dist/index.json"))
                .await?;

        // Latest version is always at the top
        if initial_version == "node" || initial_version == "latest" {
            candidate = Some(&manifest[0].version);

        // Stable version is the first with an LTS
        } else if initial_version == "stable"
            || initial_version == "lts-*"
            || initial_version == "lts/*"
        {
            for dist in &manifest {
                if let NodeLTS::Name(_) = &dist.lts {
                    candidate = Some(&dist.version);
                    break;
                }
            }

            // Find the first version with a matching LTS
        } else if initial_version.starts_with("lts-") || initial_version.starts_with("lts/") {
            let lts_name = &initial_version[4..];

            for dist in &manifest {
                if let NodeLTS::Name(lts) = &dist.lts {
                    if lts.to_lowercase() == lts_name.to_lowercase() {
                        candidate = Some(&dist.version);
                        break;
                    }
                }
            }

            if candidate.is_none() {
                return Err(ProtoError::VersionUnknownAlias(initial_version));
            }

            // Find the first version with a matching alias
        } else if is_version_alias(&initial_version) {
            for dist in &manifest {
                if let NodeLTS::Name(lts) = &dist.lts {
                    if lts.to_lowercase() == initial_version.to_lowercase() {
                        candidate = Some(&dist.version);
                        break;
                    }
                }
            }

            if candidate.is_none() {
                return Err(ProtoError::VersionUnknownAlias(initial_version));
            }

            // An explicit version? Support optional minor and patch
        } else {
            for dist in &manifest {
                if dist.version.starts_with(&add_v_prefix(&initial_version)) {
                    candidate = Some(&dist.version);
                    break;
                }
            }
        }

        let Some(candidate) = candidate else {
            return Err(ProtoError::VersionResolveFailed(initial_version))
        };

        let version = parse_version(candidate)?.to_string();

        debug!(target: self.get_log_target(), "Resolved to {}", version);

        self.version = version.clone();

        Ok(version)
    }
}
