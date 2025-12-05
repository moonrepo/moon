use std::sync::OnceLock;

static INSTANCE: OnceLock<FeatureFlags> = OnceLock::new();

pub enum Flag {}

#[derive(Default)]
pub struct FeatureFlags {}

impl FeatureFlags {
    pub fn instance() -> &'static FeatureFlags {
        INSTANCE.get_or_init(FeatureFlags::default)
    }

    pub fn is_enabled(&self, _flag: Flag) -> bool {
        true
    }

    pub fn set(self, _flag: Flag, _value: bool) -> Self {
        self
    }

    pub fn register(self) {
        let _ = INSTANCE.set(self);
    }
}
