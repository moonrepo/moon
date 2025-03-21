use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

static INSTANCE: OnceLock<FeatureFlags> = OnceLock::new();

pub enum Flag {
    FastGlobWalk,
}

#[derive(Default)]
pub struct FeatureFlags {
    fast_glob_walk: AtomicBool,
}

impl FeatureFlags {
    pub fn session() -> &'static FeatureFlags {
        INSTANCE.get_or_init(|| FeatureFlags::default())
    }

    pub fn is_enabled(&self, flag: Flag) -> bool {
        let atomic = match flag {
            Flag::FastGlobWalk => &self.fast_glob_walk,
        };

        atomic.load(Ordering::Acquire)
    }

    pub fn set(mut self, flag: Flag, value: bool) -> Self {
        match flag {
            Flag::FastGlobWalk => self.fast_glob_walk = value.into(),
        };

        self
    }

    pub fn register(self) {
        let _ = INSTANCE.set(self);
    }
}
