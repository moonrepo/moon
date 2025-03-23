use std::env;
use std::sync::OnceLock;

static INSTANCE: OnceLock<GlobalEnvBag> = OnceLock::new();

#[derive(Default)]
pub struct GlobalEnvBag {
    vars: scc::HashMap<String, String>,
}

impl GlobalEnvBag {
    pub fn instance() -> &'static GlobalEnvBag {
        INSTANCE.get_or_init(|| GlobalEnvBag::new(env::vars()))
    }

    pub fn new<I>(vars: I) -> Self
    where
        I: IntoIterator<Item = (String, String)>,
    {
        Self {
            vars: scc::HashMap::from_iter(vars),
        }
    }

    pub fn has<K>(&self, key: K) -> bool
    where
        K: AsRef<str>,
    {
        self.vars.contains(key.as_ref().into())
    }

    pub fn get<K>(&self, key: K) -> Option<String>
    where
        K: AsRef<str>,
    {
        self.vars
            .read(key.as_ref().into(), |_, value| value.clone())
    }

    pub fn set<K, V>(&self, key: K, value: V)
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let _ = self.vars.insert(key.as_ref().into(), value.as_ref().into());
    }
}
