pub mod actions;
mod hasher;

pub use hasher::SystemTargetHasher;
use moon_contract::PlatformBridge;

pub struct SystemPlatformBridge;

impl PlatformBridge for SystemPlatformBridge {}
