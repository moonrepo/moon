use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::platform::Platform;
use moon_config::PlatformType;
use moon_tool::ToolError;
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;

static PLATFORM_REGISTRY: Lazy<RwLock<PlatformManager>> =
    Lazy::new(|| RwLock::new(PlatformManager::default()));

pub type BoxedPlatform = Box<dyn Platform>;

#[derive(Debug, Default)]
pub struct PlatformManager {
    cache: FxHashMap<PlatformType, BoxedPlatform>,
}

impl PlatformManager {
    pub fn read() -> RwLockReadGuard<'static, PlatformManager> {
        PLATFORM_REGISTRY
            .read()
            .expect("Failed to acquire read access to platforms registry!")
    }

    pub fn write() -> RwLockWriteGuard<'static, PlatformManager> {
        PLATFORM_REGISTRY
            .write()
            .expect("Failed to acquire write access to platforms registry!")
    }

    pub fn find<P>(&self, predicate: P) -> Option<&BoxedPlatform>
    where
        P: Fn(&&BoxedPlatform) -> bool,
    {
        self.cache.values().find(predicate)
    }

    pub fn get<T: Into<PlatformType>>(&self, type_of: T) -> miette::Result<&BoxedPlatform> {
        let type_of = type_of.into();

        self.cache
            .get(&type_of)
            .ok_or_else(|| ToolError::UnsupportedPlatform(type_of.to_string()).into())
    }

    pub fn get_mut<T: Into<PlatformType>>(
        &mut self,
        type_of: T,
    ) -> miette::Result<&mut BoxedPlatform> {
        let type_of = type_of.into();

        self.cache
            .get_mut(&type_of)
            .ok_or_else(|| ToolError::UnsupportedPlatform(type_of.to_string()).into())
    }

    pub fn list(&self) -> std::collections::hash_map::Values<PlatformType, BoxedPlatform> {
        self.cache.values()
    }

    pub fn list_mut(
        &mut self,
    ) -> std::collections::hash_map::ValuesMut<PlatformType, BoxedPlatform> {
        self.cache.values_mut()
    }

    pub fn register(&mut self, type_of: PlatformType, platform: BoxedPlatform) {
        self.cache.insert(type_of, platform);
    }
}
