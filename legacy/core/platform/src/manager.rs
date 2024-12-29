use crate::platform::Platform;
use moon_common::Id;
use moon_tool::ToolError;
use rustc_hash::FxHashMap;
use std::sync::OnceLock;

static mut PLATFORM_REGISTRY: OnceLock<PlatformManager> = OnceLock::new();

pub type BoxedPlatform = Box<dyn Platform>;

#[derive(Default)]
pub struct PlatformManager {
    cache: FxHashMap<Id, BoxedPlatform>,
}

impl PlatformManager {
    pub fn read() -> &'static PlatformManager {
        #[allow(static_mut_refs)]
        unsafe {
            PLATFORM_REGISTRY.get_or_init(PlatformManager::default)
        }
    }

    pub fn write() -> &'static mut PlatformManager {
        {
            // Initialize if it hasn't been
            PlatformManager::read();
        }

        #[allow(static_mut_refs)]
        unsafe {
            PLATFORM_REGISTRY.get_mut().unwrap()
        }
    }

    pub fn find<P>(&self, predicate: P) -> Option<&BoxedPlatform>
    where
        P: Fn(&&BoxedPlatform) -> bool,
    {
        self.cache.values().find(predicate)
    }

    pub fn get_by_toolchain(&self, id: &Id) -> miette::Result<&BoxedPlatform> {
        self.cache.get(id).ok_or_else(|| {
            ToolError::UnsupportedToolchains {
                ids: vec![id.to_string()],
            }
            .into()
        })
    }

    pub fn get_by_toolchains(&self, ids: &[Id]) -> miette::Result<&BoxedPlatform> {
        for id in ids {
            if let Some(platform) = self.cache.get(id) {
                return Ok(platform);
            }
        }

        Err(ToolError::UnsupportedToolchains {
            ids: ids.iter().map(|tc| tc.to_string()).collect(),
        }
        .into())
    }

    pub fn get_by_toolchain_mut(&mut self, id: &Id) -> miette::Result<&mut BoxedPlatform> {
        self.cache.get_mut(id).ok_or_else(|| {
            ToolError::UnsupportedToolchains {
                ids: vec![id.to_string()],
            }
            .into()
        })
    }

    pub fn enabled(&self) -> std::collections::hash_map::Keys<Id, BoxedPlatform> {
        self.cache.keys()
    }

    pub fn list(&self) -> std::collections::hash_map::Values<Id, BoxedPlatform> {
        self.cache.values()
    }

    pub fn list_mut(&mut self) -> std::collections::hash_map::ValuesMut<Id, BoxedPlatform> {
        self.cache.values_mut()
    }

    pub fn register(&mut self, id: Id, platform: BoxedPlatform) {
        self.cache.insert(id, platform);
    }

    pub fn reset(&mut self) {
        self.cache.clear();
    }
}
