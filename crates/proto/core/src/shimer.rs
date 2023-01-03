use proto_error::ProtoError;
use std::path::Path;

use crate::get_root;

#[async_trait::async_trait]
pub trait Shimable<'tool>: Send + Sync {
    /// ???
    async fn create_shims(&self) -> Result<(), ProtoError>;
}

pub struct ShimBuilder<'tool> {
    name: &'tool str,
    bin_path: &'tool Path,
    install_dir: Option<&'tool Path>,
    version: Option<&'tool str>,
}

impl<'tool> ShimBuilder<'tool> {
    pub fn new(name: &'tool str, bin_path: &'tool Path) -> Self {
        ShimBuilder {
            name,
            bin_path,
            install_dir: None,
            version: None,
        }
    }

    pub fn install_dir(&mut self, path: &'tool Path) -> &mut Self {
        self.install_dir = Some(path);
        self
    }

    pub fn version(&mut self, version: &'tool str) -> &mut Self {
        self.version = Some(version);
        self
    }

    pub fn build(&self) -> Result<String, ProtoError> {
        let constant_name = self.name.to_uppercase();

        let mut template = vec![];
        template.push("#!/usr/bin/env bash".into());
        template.push("set -e".into());
        template.push("[ -n \"$PROTO_DEBUG\" ] && set -x".into());
        template.push("".into());

        template.push(format!(
            "export PROTO_ROOT=\"{}\"",
            get_root()?.to_string_lossy()
        ));

        if let Some(install_dir) = &self.install_dir {
            template.push(format!(
                "export PROTO_{}_DIR=\"{}\"",
                constant_name,
                install_dir.to_string_lossy()
            ));
        }
        if let Some(version) = &self.version {
            template.push(format!(
                "export PROTO_{}_VERSION=\"{}\"",
                constant_name, version
            ));
        }

        template.push("".into());
        template.push(format!(
            "exec \"{}\" \"$@\"",
            self.bin_path.to_string_lossy()
        ));

        Ok(template.join("\n"))
    }
}
