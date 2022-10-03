pub mod actions;
mod hasher;

pub use hasher::SystemTargetHasher;
use moon_contract::{Platform, SupportedPlatform};

#[derive(Default)]
pub struct SystemPlatform;

impl Platform for SystemPlatform {
    fn is(&self, platform: &SupportedPlatform) -> bool {
        matches!(platform, SupportedPlatform::System)
    }
}
