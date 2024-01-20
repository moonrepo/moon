use miette::miette;
use moon_common::consts::CONFIG_DIRNAME;
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct MoonEnvironment {
    pub cwd: PathBuf,
    pub id_file: PathBuf,
    pub plugins_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub home: PathBuf,       // ~
    pub store_root: PathBuf, // ~/.moon
    pub version: String,
    pub workspace_root: PathBuf,
}

impl MoonEnvironment {
    pub fn new() -> miette::Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| {
            miette!(
                code = "env::missing_home",
                "Unable to determine your home directory."
            )
        })?;

        let cwd = env::current_dir().map_err(|_| {
            miette!(
                code = "env::missing_cwd",
                "Unable to determine your current working directory."
            )
        })?;

        let store_root = if let Ok(root) = env::var("MOON_HOME") {
            root.into()
        } else {
            home.join(CONFIG_DIRNAME)
        };

        debug!(store = ?store_root, "Creating moon environment, detecting store");

        Ok(MoonEnvironment {
            id_file: store_root.join("id"),
            plugins_dir: store_root.join("plugins"),
            temp_dir: store_root.join("temp"),
            home,
            store_root,
            version: env::var("MOON_VERSION").unwrap_or_default(),
            workspace_root: cwd.clone(),
            cwd,
        })
    }

    pub fn get_virtual_paths(&self) -> BTreeMap<PathBuf, PathBuf> {
        BTreeMap::from_iter([
            (self.cwd.clone(), "/cwd".into()),
            (self.store_root.clone(), "/moon".into()),
            (self.home.clone(), "/userhome".into()),
            (self.workspace_root.clone(), "/workspace".into()),
        ])
    }
}

impl AsRef<MoonEnvironment> for MoonEnvironment {
    fn as_ref(&self) -> &MoonEnvironment {
        self
    }
}
