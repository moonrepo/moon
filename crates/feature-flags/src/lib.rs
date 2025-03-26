mod wrappers;

use std::sync::OnceLock;

pub use wrappers::*;

static INSTANCE: OnceLock<FeatureFlags> = OnceLock::new();

pub enum Flag {
    FastGlobWalk,
}

#[derive(Default)]
pub struct FeatureFlags {
    fast_glob_walk: bool,
}

impl FeatureFlags {
    pub fn instance() -> &'static FeatureFlags {
        INSTANCE.get_or_init(FeatureFlags::default)
    }

    pub fn is_enabled(&self, flag: Flag) -> bool {
        match flag {
            Flag::FastGlobWalk => self.fast_glob_walk,
        }
    }

    pub fn set(mut self, flag: Flag, value: bool) -> Self {
        match flag {
            Flag::FastGlobWalk => self.fast_glob_walk = value,
        };

        self
    }

    pub fn register(self) {
        let _ = INSTANCE.set(self);
    }
}
