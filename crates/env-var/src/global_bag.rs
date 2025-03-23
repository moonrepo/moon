use std::env;
use std::ffi::{OsStr, OsString};
use std::sync::OnceLock;

static INSTANCE: OnceLock<GlobalEnvBag> = OnceLock::new();

#[derive(Default)]
pub struct GlobalEnvBag {
    inherited: scc::HashMap<OsString, OsString>,
    configured: scc::HashMap<OsString, OsString>,
}

impl GlobalEnvBag {
    pub fn instance() -> &'static GlobalEnvBag {
        INSTANCE.get_or_init(|| GlobalEnvBag {
            inherited: scc::HashMap::from_iter(env::vars_os()),
            configured: scc::HashMap::new(),
        })
    }

    pub fn has<K>(&self, key: K) -> bool
    where
        K: AsRef<str>,
    {
        let key = OsStr::new(key.as_ref());

        self.inherited.contains(key) || self.configured.contains(key)
    }

    pub fn get<K>(&self, key: K) -> Option<String>
    where
        K: AsRef<str>,
    {
        let key = OsStr::new(key.as_ref());

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
        let _ = self
            .configured
            .insert(key.as_ref().into(), value.as_ref().into());
    }
}
