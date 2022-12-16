use std::path::Path;

use crate::platform::Platform;
use moon_config::{PlatformType, ProjectLanguage};
use rustc_hash::FxHashMap;

pub type BoxedPlatform = Box<dyn Platform>;

#[derive(Debug, Default)]
pub struct PlatformManager {
    cache: FxHashMap<PlatformType, BoxedPlatform>,
}

impl PlatformManager {
    pub fn detect_project_language(&self, root: &Path) -> ProjectLanguage {
        for platform in self.list() {
            if let Some(language) = platform.detect_project_language(root) {
                return language;
            }
        }

        ProjectLanguage::Unknown
    }

    pub fn find<P>(&self, predicate: P) -> Option<&BoxedPlatform>
    where
        P: Fn(&&BoxedPlatform) -> bool,
    {
        self.cache.values().find(predicate)
    }

    pub fn get<T: Into<PlatformType>>(&self, type_of: T) -> Option<&BoxedPlatform> {
        let type_of = type_of.into();

        self.cache.get(&type_of)
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
