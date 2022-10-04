pub mod actions;
mod hasher;

pub use hasher::SystemTargetHasher;
use moon_contract::{Platform, Runtime};

#[derive(Default)]
pub struct SystemPlatform;

impl Platform for SystemPlatform {
    fn is(&self, platform: &Runtime) -> bool {
        matches!(platform, Runtime::System)
    }
}
