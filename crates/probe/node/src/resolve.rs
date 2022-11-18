use crate::tool::NodeLanguage;
use probe_core::{
    async_trait, load_versions_manifest, parse_version, Probe, ProbeError, Resolvable,
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
#[serde(untagged)]
enum NodeLTS {
    Name(String),
    State(bool),
}

#[derive(Deserialize)]
struct NodeDistVersion {
    date: String,
    files: Vec<String>,
    lts: NodeLTS,
    npm: String,
    security: bool,
    version: String, // Starts with v
    #[serde(flatten)]
    versions: HashMap<String, String>,
}

#[async_trait]
impl<'tool> Resolvable<'tool, Probe> for NodeLanguage<'tool> {
    fn get_resolved_version(&self) -> &str {
        &self.version
    }

    async fn resolve_version(&self, initial_version: &str) -> Result<String, ProbeError> {
        let mut possible_version = None;
        let mut initial_version = initial_version.to_lowercase();
        let manifest: Vec<NodeDistVersion> =
            load_versions_manifest("https://nodejs.org/dist/index.json").await?;

        // Latest version is always at the top
        if initial_version == "node" || initial_version == "latest" {
            possible_version = Some(&manifest[0].version);

        // Stable version is the first with an LTS
        } else if initial_version == "stable"
            || initial_version == "lts-*"
            || initial_version == "lts/*"
        {
            for dist in &manifest {
                if let NodeLTS::Name(_) = &dist.lts {
                    possible_version = Some(&dist.version);
                    break;
                }
            }

            // Find the first version with a matching LTS
        } else if initial_version.starts_with("lts-") || initial_version.starts_with("lts/") {
            let lts_name = &initial_version[4..];

            for dist in &manifest {
                if let NodeLTS::Name(lts) = &dist.lts {
                    if lts.to_lowercase() == lts_name.to_lowercase() {
                        possible_version = Some(&dist.version);
                        break;
                    }
                }
            }

            // An explicit version? Support optional minor and patch
        } else {
            if !initial_version.starts_with('v') {
                initial_version = format!("v{}", initial_version)
            };

            for dist in &manifest {
                if dist.version.starts_with(&initial_version) {
                    possible_version = Some(&dist.version);
                    break;
                }
            }
        }

        let Some(possible_version) = possible_version else {
            return Err(ProbeError::VersionResolveFailed(initial_version))
        };

        Ok(parse_version(possible_version)?.to_string())
    }
}
