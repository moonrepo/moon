use std::env;
use std::ffi::{OsStr, OsString};
use std::sync::OnceLock;

static INSTANCE: OnceLock<GlobalEnvBag> = OnceLock::new();

#[derive(Default)]
pub struct GlobalEnvBag {
    inherited: scc::HashMap<OsString, OsString>,
    configured: scc::HashMap<OsString, OsString>,
    removed: scc::HashSet<OsString>,
}

impl GlobalEnvBag {
    pub fn instance() -> &'static GlobalEnvBag {
        INSTANCE.get_or_init(|| GlobalEnvBag {
            inherited: scc::HashMap::from_iter(env::vars_os()),
            configured: scc::HashMap::new(),
            removed: scc::HashSet::new(),
        })
    }

    pub fn has<K>(&self, key: K) -> bool
    where
        K: AsRef<OsStr>,
    {
        let key = key.as_ref();

        self.inherited.contains(key) || self.configured.contains(key)
    }

    pub fn get<K>(&self, key: K) -> Option<String>
    where
        K: AsRef<OsStr>,
    {
        let key = key.as_ref();

        self.configured
            .read(key, |_, value| value.to_owned())
            .or_else(|| self.inherited.read(key, |_, value| value.to_owned()))
            .map(|value| value.to_string_lossy().to_string())
    }

    pub fn set<K, V>(&self, key: K, value: V)
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

        let _ = self.configured.insert(key.into(), value.into());

        // These need to always be propagated to the parent process
        if key.to_str().is_some_and(|k| {
            k.starts_with("PROTO")
                || k.starts_with("STARBASE")
                || k.starts_with("WARPGATE")
                || k == "PATH"
        }) {
            unsafe {
                env::set_var(key, value);
            };
        }
    }

    pub fn remove<K>(&self, key: K)
    where
        K: AsRef<OsStr>,
    {
        let key = key.as_ref();

        self.inherited.remove(key);
        self.configured.remove(key);

        let _ = self.removed.insert(key.into());
    }
}
