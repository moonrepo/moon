use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

static INSTANCE: OnceLock<FeatureFlags> = OnceLock::new();

pub enum Flag {
    FastGlobs,
}

#[derive(Default)]
pub struct FeatureFlags {
    pub fast_globs: AtomicBool,
}

impl FeatureFlags {
    pub fn session() -> &'static FeatureFlags {
        INSTANCE.get_or_init(|| FeatureFlags::default())
    }

    pub fn register(self) {
        let _ = INSTANCE.set(self);
    }

    pub fn is_enabled(&self, flag: Flag) -> bool {
        let atomic = match flag {
            Flag::FastGlobs => &self.fast_globs,
        };

        atomic.load(Ordering::Acquire)
    }
}
