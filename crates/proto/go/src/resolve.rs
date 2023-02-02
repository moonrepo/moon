use crate::GoLanguage;
use core::str;
use lenient_semver::Version;
use log::debug;
use proto_core::{
    async_trait, Describable, ProtoError, Resolvable, VersionManifest, VersionManifestEntry,
};
use std::collections::BTreeMap;
use std::process::Command;

trait BaseVersion {
    fn base_version(&self) -> String;
}

impl<'a> BaseVersion for Version<'a> {
    fn base_version(&self) -> String {
        format!("{}.{}", self.major, self.minor)
    }
}

#[async_trait]
impl Resolvable<'_> for GoLanguage {
    fn get_resolved_version(&self) -> &str {
        let v = self.version.as_ref().unwrap();
        match v.strip_suffix(".0") {
            Some(s) => s,
            None => v,
        }
    }

    async fn load_manifest(&self) -> Result<VersionManifest, ProtoError> {
        let mut alias_max = BTreeMap::new();
        let mut latest = Version::new(0, 0, 0);
        let mut aliases = BTreeMap::new();
        let mut versions = BTreeMap::new();

        let output = match Command::new("git")
            .args(["ls-remote", "--tags", "https://github.com/golang/go/"])
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                return Err(ProtoError::DownloadFailed(
                    "could not list versions from https://github.com/golang/go/".into(),
                    e.to_string(),
                ));
            }
        };

        let raw = match str::from_utf8(&output.stdout) {
            Ok(o) => o,
            Err(e) => {
                return Err(ProtoError::DownloadFailed(
                    "failed to read output from github".into(),
                    e.to_string(),
                ));
            }
        };

        for line in raw.split('\n') {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 2 {
                continue;
            }

            let tag: Vec<&str> = parts[1].split('/').collect();
            if tag.len() < 3 {
                continue;
            }

            if tag[2].starts_with("go") {
                let ver_str = tag[2].strip_prefix("go").unwrap();

                if let Ok(ver) = Version::parse(ver_str) {
                    let entry = VersionManifestEntry {
                        alias: None,
                        version: String::from(ver_str),
                    };
                    let base_version = ver.base_version();

                    if &latest < &ver {
                        latest = ver.clone();
                    }

                    let current: Option<&Version> = alias_max.get(&base_version);
                    match current {
                        Some(current_version) => {
                            if current_version < &ver {
                                aliases.insert(base_version.clone(), entry.version.clone());
                                alias_max.insert(base_version, ver);
                            }
                        }
                        None => {
                            aliases.insert(base_version.clone(), entry.version.clone());
                            alias_max.insert(base_version, ver);
                        }
                    }
                    versions.insert(entry.version.clone(), entry);
                }
            }
        }

        aliases.insert("latest".into(), latest.to_string());

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
        let candidate = if initial_version.contains("rc") || initial_version.contains("beta") {
            manifest.get_version(&initial_version)?
        } else {
            match manifest.find_version_from_alias(&initial_version) {
                Ok(found) => found,
                _ => manifest.find_version(&initial_version)?,
            }
        };

        debug!(target: self.get_log_target(), "Resolved to {}", candidate);

        self.version = Some(candidate.clone());

        Ok(candidate.to_owned())
    }
}
