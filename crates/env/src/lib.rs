use miette::miette;
use moon_common::consts::CONFIG_DIRNAME;
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Default, Clone)]
pub struct MoonEnvironment {
    pub id_file: PathBuf,
    pub plugins_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub templates_dir: PathBuf,
    pub home: PathBuf,       // ~
    pub store_root: PathBuf, // ~/.moon
    pub test_only: bool,
    #[deprecated]
    pub version: String,
    pub working_dir: PathBuf,
    pub workspace_root: PathBuf,
}

impl MoonEnvironment {
    pub fn new() -> miette::Result<Self> {
        let store_root = if let Ok(root) = env::var("MOON_HOME") {
            root.into()
        } else {
            dirs::home_dir()
                .ok_or_else(|| {
                    miette!(
                        code = "env::missing_home",
                        "Unable to determine your home directory."
                    )
                })?
                .join(CONFIG_DIRNAME)
        };

        Self::from(store_root)
    }

    pub fn from<P: AsRef<Path>>(root: P) -> miette::Result<Self> {
        let store_root = root.as_ref();

        debug!(store = ?store_root, "Creating moon environment, detecting store");

        Ok(MoonEnvironment {
            id_file: store_root.join("id"),
            plugins_dir: store_root.join("plugins"),
            temp_dir: store_root.join("temp"),
            templates_dir: store_root.join("templates"),
            home: dirs::home_dir().unwrap(),
            store_root: store_root.to_owned(),
            test_only: false,
            version: env::var("MOON_VERSION").unwrap_or_default(),
            working_dir: PathBuf::new(),
            workspace_root: PathBuf::new(),
        })
    }

    pub fn new_testing(sandbox: &Path) -> Self {
        let mut env = Self::from(sandbox.join(".moon")).unwrap();
        env.working_dir = sandbox.to_path_buf();
        env.workspace_root = sandbox.to_path_buf();
        env.home = sandbox.join(".home");
        env.test_only = true;
        env
    }

    pub fn get_virtual_paths(&self) -> BTreeMap<PathBuf, PathBuf> {
        BTreeMap::from_iter([
            (self.working_dir.clone(), "/cwd".into()),
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
