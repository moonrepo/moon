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
    pub home: PathBuf, // ~
    pub root: PathBuf, // ~/.moon
}

impl MoonEnvironment {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Unable to determine home directory!");

        let root = if let Ok(root) = env::var("MOON_HOME") {
            root.into()
        } else {
            home.join(CONFIG_DIRNAME)
        };

        debug!(store = ?root, "Creating moon environment");

        MoonEnvironment {
            cwd: env::current_dir().expect("Unable to determine current working directory!"),
            id_file: root.join("id"),
            plugins_dir: root.join("plugins"),
            temp_dir: root.join("temp"),
            home,
            root,
        }
    }

    pub fn get_virtual_paths(&self) -> BTreeMap<PathBuf, PathBuf> {
        BTreeMap::from_iter([
            (self.cwd.clone(), "/workspace".into()),
            (self.root.clone(), "/moon".into()),
            (self.home.clone(), "/userhome".into()),
        ])
    }
}

impl AsRef<MoonEnvironment> for MoonEnvironment {
    fn as_ref(&self) -> &MoonEnvironment {
        self
    }
}
