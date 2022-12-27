use crate::platform::Platform;
use moon_config::PlatformType;
use moon_error::MoonError;
use rustc_hash::FxHashMap;

pub type BoxedPlatform = Box<dyn Platform>;

#[derive(Debug, Default)]
pub struct PlatformManager {
    cache: FxHashMap<PlatformType, BoxedPlatform>,
}

impl PlatformManager {
    pub fn find<P>(&self, predicate: P) -> Option<&BoxedPlatform>
    where
        P: Fn(&&BoxedPlatform) -> bool,
    {
        self.cache.values().find(predicate)
    }

    pub fn get<T: Into<PlatformType>>(&self, type_of: T) -> Result<&BoxedPlatform, MoonError> {
        let type_of = type_of.into();

        self.cache
            .get(&type_of)
            .ok_or_else(|| MoonError::UnsupportedPlatform(type_of.to_string()))
    }

    pub fn get_mut<T: Into<PlatformType>>(
        &mut self,
        type_of: T,
    ) -> Result<&mut BoxedPlatform, MoonError> {
        let type_of = type_of.into();

        self.cache
            .get_mut(&type_of)
            .ok_or_else(|| MoonError::UnsupportedPlatform(type_of.to_string()))
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
