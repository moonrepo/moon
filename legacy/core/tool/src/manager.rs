use crate::errors::ToolError;
use crate::tool::Tool;
use moon_toolchain::{Runtime, RuntimeReq};
use proto_core::UnresolvedVersionSpec;
use rustc_hash::FxHashMap;

pub struct ToolManager<T: Tool> {
    cache: FxHashMap<RuntimeReq, T>,
    default_req: RuntimeReq,
    runtime: Runtime,
}

impl<T: Tool> ToolManager<T> {
    pub fn new(runtime: Runtime) -> Self {
        ToolManager {
            cache: FxHashMap::default(),
            default_req: runtime.requirement.clone(),
            runtime,
        }
    }

    pub fn get(&self) -> miette::Result<&T> {
        self.get_for_version(&self.default_req)
    }

    pub fn get_for_version<V: AsRef<RuntimeReq>>(&self, req: V) -> miette::Result<&T> {
        let req = req.as_ref();

        if !self.has(req) {
            return Err(ToolError::UnknownTool(format!("{} {}", self.runtime, req)).into());
        }

        Ok(self.cache.get(req).unwrap())
    }

    pub fn has(&self, req: &RuntimeReq) -> bool {
        self.cache.contains_key(req)
    }

    pub fn register(&mut self, req: &RuntimeReq, tool: T) {
        // Nothing exists in the cache yet, so this tool must be the top-level
        // workspace tool. If so, update the default version within the platform.
        if self.default_req.is_global() && !req.is_global() {
            self.default_req = req.to_owned();
        }

        self.cache.insert(req.to_owned(), tool);
    }

    pub async fn setup(
        &mut self,
        req: &RuntimeReq,
        last_versions: &mut FxHashMap<String, UnresolvedVersionSpec>,
    ) -> miette::Result<u8> {
        match self.cache.get_mut(req) {
            Some(cache) => Ok(cache.setup(last_versions).await?),
            None => Err(ToolError::UnknownTool(self.runtime.to_string()).into()),
        }
    }

    pub async fn teardown(&mut self, req: &RuntimeReq) -> miette::Result<()> {
        if let Some(mut tool) = self.cache.remove(req) {
            tool.teardown().await?;
        }

        Ok(())
    }

    pub async fn teardown_all(&mut self) -> miette::Result<()> {
        for (_, mut tool) in self.cache.drain() {
            tool.teardown().await?;
        }

        Ok(())
    }
}
