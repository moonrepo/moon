use crate::GoLanguage;
use log::debug;
use lenient_semver::Version;
use proto_core::{
    async_trait, load_versions_manifest, parse_version, remove_v_prefix, Describable, ProtoError,
    Resolvable, VersionManifest, VersionManifestEntry,
};
use core::str;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::process::Command;

#[derive(Deserialize)]
#[serde(untagged)]
enum GoLTS {
    Name(String),
    State(bool),
}

#[derive(Deserialize)]
struct GoDistVersion {
    lts: GoLTS,
    version: String, // Starts with v
}

trait GoBaseVersion {
    fn go_base_version(&self) -> String;
}

impl<'a> GoBaseVersion for Version<'a> {
    fn go_base_version(&self) -> String {
        format!("{}.{}", self.major, self.minor)
    }
}

#[async_trait]
impl Resolvable<'_> for GoLanguage {
    fn get_resolved_version(&self) -> &str {
        self.version.as_ref().unwrap()
    }

    async fn load_manifest(&self) -> Result<VersionManifest, ProtoError> {
        let mut aliases = BTreeMap::new();
        let mut versions = BTreeMap::new();

        let output = Command::new("git")
                .args(["ls-remote", "--tags", "https://github.com/golang/go/"])
                .output()
                .expect("failed to execute process");

        let raw = str::from_utf8(&output.stdout).expect("could not parse output from github");

        for line in raw.split("\n") {
            let parts: Vec<&str> = line.split("\t").collect();
            if parts.len() < 2 {
                continue
            }

            let tag: Vec<&str> = parts[1].split("/").collect();
            if tag.len() < 3 {
                continue
            }

            if tag[2].starts_with("go") {
                let ver_str = tag[2].strip_prefix("go").unwrap();

                match Version::parse(ver_str) {
                    Ok(ver) => {
                        let entry = VersionManifestEntry {
                            alias: None,
                            version: String::from(ver_str),
                        };
                        aliases.insert(ver.go_base_version(), entry.version.clone());
                        versions.insert(entry.version.clone(), entry);
                    }
                    Err(_) => {}
                }
            }
        };

        Ok(VersionManifest { aliases, versions })
    }

    async fn resolve_version(&mut self, initial_version: &str) -> Result<String, ProtoError> {
        if let Some(version) = &self.version {
            return Ok(version.to_owned());
        }

        let initial_version = initial_version.to_lowercase();

        debug!(
            target: self.get_log_target(),
            "Resolving a semantic version for \"{}\"",
            initial_version,
        );

        let manifest = self.load_manifest().await?;
        let candidate;

        if initial_version.contains("rc") || initial_version.contains("beta") {
            candidate = manifest.get_version(&initial_version)?;
        } else {
            candidate = match manifest.find_version_from_alias(&initial_version) {
                Ok(found) => found,
                _ => manifest.find_version(&initial_version)?
            }
        }

        debug!(target: self.get_log_target(), "Resolved to {}", candidate);

        self.version = Some(candidate.clone());

        Ok(candidate.to_owned())
    }
}
