pub mod actions;
mod hasher;

pub use hasher::SystemTargetHasher;
use moon_contract::Platform;

#[derive(Default)]
pub struct SystemPlatform;

impl Platform for SystemPlatform {}
