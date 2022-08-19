pub mod actions;
mod hasher;

pub use hasher::SystemTargetHasher;
use moon_contract::Platform;

pub struct SystemPlatform;

impl SystemPlatform {
    pub fn new() -> Self {
        SystemPlatform {}
    }
}

impl Platform for SystemPlatform {}
