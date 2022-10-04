pub mod actions;
mod hasher;

pub use hasher::SystemTargetHasher;
use moon_contract::{Platform, Runtime};

#[derive(Default)]
pub struct SystemPlatform;

impl Platform for SystemPlatform {
    fn is(&self, runtime: &Runtime) -> bool {
        matches!(runtime, Runtime::System)
    }
}
