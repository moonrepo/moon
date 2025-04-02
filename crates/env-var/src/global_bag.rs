use std::env;
use std::ffi::{OsStr, OsString};
use std::sync::OnceLock;

static INSTANCE: OnceLock<GlobalEnvBag> = OnceLock::new();

#[derive(Default)]
pub struct GlobalEnvBag {
    inherited: scc::HashMap<OsString, OsString>,
    added: scc::HashMap<OsString, OsString>,
    removed: scc::HashSet<OsString>,
}

impl GlobalEnvBag {
    pub fn instance() -> &'static GlobalEnvBag {
        INSTANCE.get_or_init(|| GlobalEnvBag {
            inherited: scc::HashMap::from_iter(env::vars_os()),
            added: scc::HashMap::new(),
            removed: scc::HashSet::new(),
        })
    }

    pub fn has<K>(&self, key: K) -> bool
    where
        K: AsRef<OsStr>,
    {
        let key = key.as_ref();

        self.inherited.contains(key) || self.added.contains(key)
    }

    pub fn get<K>(&self, key: K) -> Option<String>
    where
        K: AsRef<OsStr>,
    {
        self.get_as(key, |value| value.to_string_lossy().to_string())
    }

    pub fn get_as<K, T>(&self, key: K, op: impl Fn(&OsString) -> T) -> Option<T>
    where
        K: AsRef<OsStr>,
    {
        let key = key.as_ref();

        if let Some(value) = self
            .added
            .read(key, |_, value| op(value))
            .or_else(|| self.inherited.read(key, |_, value| op(value)))
        {
            return Some(value);
        }

        // If it doesn't exist in our current bag, let's check the process,
        // as it may have been inserted after the fact
        if let Some(value) = env::var_os(key) {
            let as_value = op(&value);

            let _ = self.inherited.insert(key.into(), value);

            return Some(as_value);
        }

        None
    }

    pub fn set<K, V>(&self, key: K, value: V)
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

        self.added.upsert(key.into(), value.into());

        // These need to always be propagated to the parent process
        if key.to_str().is_some_and(|k| {
            k.starts_with("PROTO")
                || k.starts_with("STARBASE")
                || k.starts_with("WARPGATE")
                || k.contains("COLOR")
                || k == "PATH"
                || k == "MOON_VERSION"
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
        self.added.remove(key);

        let _ = self.removed.insert(key.into());
    }

    pub fn list(&self, mut op: impl FnMut(&OsString, &OsString)) {
        self.inherited.scan(|k, v| {
            op(k, v);
        });
        self.added.scan(|k, v| {
            op(k, v);
        });
    }

    pub fn list_added(&self, op: impl FnMut(&OsString, &OsString)) {
        self.added.scan(op);
    }

    pub fn list_removed(&self, op: impl FnMut(&OsString)) {
        self.removed.scan(op);
    }

    pub fn should_debug_process_env(&self) -> bool {
        self.get_as("MOON_DEBUG_PROCESS_ENV", as_bool)
            .unwrap_or_default()
    }

    pub fn should_debug_process_input(&self) -> bool {
        self.get_as("MOON_DEBUG_PROCESS_INPUT", as_bool)
            .unwrap_or_default()
    }

    pub fn should_debug_remote(&self) -> bool {
        self.get_as("MOON_DEBUG_REMOTE", as_bool)
            .unwrap_or_default()
    }

    pub fn should_debug_wasm(&self) -> bool {
        self.get_as("MOON_DEBUG_WASM", as_bool).unwrap_or_default()
    }
}

pub fn as_bool(value: &OsString) -> bool {
    value
        .to_str()
        .map(|value| value.to_lowercase())
        .is_some_and(|value| {
            value == "1" || value == "true" || value == "yes" || value == "on" || value == "enable"
        })
}
