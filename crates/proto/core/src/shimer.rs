use crate::helpers::get_root;
use proto_error::ProtoError;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[async_trait::async_trait]
pub trait Shimable<'tool>: Send + Sync {
    /// ???
    async fn create_shims(&self) -> Result<(), ProtoError>;

    fn get_shim_path(&self) -> Option<&Path>;
}

fn build_shim_file(builder: &ShimBuilder) -> Result<String, ProtoError> {
    let constant_name = builder.name.to_uppercase();

    let mut template = vec![];
    template.push("#!/usr/bin/env bash".into());
    template.push("set -e".into());
    template.push("[ -n \"$PROTO_DEBUG\" ] && set -x".into());
    template.push("".into());

    template.push(format!(
        "export PROTO_ROOT=\"{}\"",
        get_root()?.to_string_lossy()
    ));

    if let Some(install_dir) = &builder.install_dir {
        template.push(format!(
            "export PROTO_{}_DIR=\"{}\"",
            constant_name,
            install_dir.to_string_lossy()
        ));
    }
    if let Some(version) = &builder.version {
        template.push(format!(
            "export PROTO_{}_VERSION=\"{}\"",
            constant_name, version
        ));
    }

    template.push("".into());

    if let Some(parent_name) = &builder.parent_name {
        template.push(format!(
            "parent=\"${{PROTO_{}_BIN:-{}}}\"",
            parent_name.to_uppercase(),
            parent_name
        ));
        template.push("".into());
        template.push(format!(
            "exec \"$parent\" \"{}\" \"$@\"",
            builder.bin_path.to_string_lossy()
        ));
    } else {
        template.push(format!(
            "exec \"{}\" \"$@\"",
            builder.bin_path.to_string_lossy()
        ));
    }

    Ok(template.join("\n"))
}

pub struct ShimBuilder {
    pub name: String,
    pub bin_path: PathBuf,
    pub install_dir: Option<PathBuf>,
    pub parent_name: Option<String>,
    pub version: Option<String>,
}

impl ShimBuilder {
    pub fn new(name: &str, bin_path: &Path) -> Self {
        ShimBuilder {
            name: name.to_owned(),
            bin_path: bin_path.to_path_buf(),
            install_dir: None,
            parent_name: None,
            version: None,
        }
    }

    pub fn dir<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.install_dir = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn parent<V: AsRef<str>>(&mut self, name: V) -> &mut Self {
        self.parent_name = Some(name.as_ref().to_owned());
        self
    }

    pub fn version<V: AsRef<str>>(&mut self, version: V) -> &mut Self {
        self.version = Some(version.as_ref().to_owned());
        self
    }

    pub fn create(&self) -> Result<(), ProtoError> {
        let shim_path = self.install_dir.as_ref().unwrap().join(&self.name);
        let handle_error =
            |e: std::io::Error| ProtoError::Fs(shim_path.to_path_buf(), e.to_string());

        fs::write(&shim_path, build_shim_file(&self)?).map_err(handle_error)?;

        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            fs::set_permissions(&shim_path, fs::Permissions::from_mode(0o755))
                .map_err(handle_error)?;
        }

        Ok(())
    }
}
