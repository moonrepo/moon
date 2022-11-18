use crate::tool::NodeLanguage;
use probe_core::{async_trait, load_versions_manifest, parse_version, ProbeError, Resolvable};
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
impl<'tool> Resolvable<'tool> for NodeLanguage<'tool> {
    fn get_resolved_version(&self) -> &str {
        &self.version
    }

    async fn resolve_version(
        &mut self,
        initial_version: &str,
        manifest_url: Option<&str>,
    ) -> Result<String, ProbeError> {
        let mut candidate = None;
        let mut initial_version = initial_version.to_lowercase();
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

            // An explicit version? Support optional minor and patch
        } else {
            if !initial_version.starts_with('v') {
                initial_version = format!("v{}", initial_version)
            };

            for dist in &manifest {
                if dist.version.starts_with(&initial_version) {
                    candidate = Some(&dist.version);
                    break;
                }
            }
        }

        let Some(candidate) = candidate else {
            return Err(ProbeError::VersionResolveFailed(initial_version))
        };

        let version = parse_version(candidate)?.to_string();

        self.version = version.clone();

        Ok(version)
    }
}
