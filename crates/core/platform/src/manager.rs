use crate::platform::Platform;
use moon_config::{PlatformType, ProjectLanguage};
use moon_error::MoonError;
use rustc_hash::FxHashMap;
use std::path::Path;

pub type BoxedPlatform = Box<dyn Platform>;

#[derive(Debug, Default)]
pub struct PlatformManager {
    cache: FxHashMap<PlatformType, BoxedPlatform>,
}

impl PlatformManager {
    pub fn detect_project_language(&self, root: &Path) -> Result<ProjectLanguage, MoonError> {
        for platform in self.list() {
            if let Some(language) = platform.is_project_language(root) {
                return Ok(language);
            }
        }

        Ok(ProjectLanguage::Unknown)
    }

    pub fn detect_task_platform(
        &self,
        command: &str,
        language: ProjectLanguage,
    ) -> Result<PlatformType, MoonError> {
        for platform in self.list() {
            if platform.is_task_command(command) {
                return Ok(platform.get_type());
            }
        }

        // Default to the platform of the project's language
        Ok(language.into())
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
