use crate::asset_api::Asset;
use std::sync::{Arc, OnceLock};

static INSTANCE: OnceLock<Arc<RemoteService>> = OnceLock::new();

pub struct RemoteService {
    pub asset: Asset,
}

impl RemoteService {
    pub fn session() -> Option<Arc<RemoteService>> {
        INSTANCE.get().cloned()
    }
}
