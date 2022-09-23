use crate::subscribers::local_cache::LocalCacheSubscriber;
use moon_contract::{Emitter, EventFlow};
use moon_error::MoonError;
use moon_workspace::Workspace;

macro_rules! handle_flow {
    ($result:expr) => {
        match $result? {
            EventFlow::Break => return Ok(EventFlow::Break),
            EventFlow::Return(value) => return Ok(EventFlow::Return(value)),
            EventFlow::Continue => {}
        };
    };
}

pub enum Event<'a> {
    TargetOutputArchive,
    TargetOutputHydrate,
    TargetOutputCheckCache(&'a Workspace, &'a str),
}

pub struct RunnerEmitter {
    local_cache: LocalCacheSubscriber,
}

impl RunnerEmitter {
    pub fn new() -> Self {
        RunnerEmitter {
            local_cache: LocalCacheSubscriber::new(),
        }
    }

    pub async fn emit<'e>(&mut self, event: Event<'e>) -> Result<EventFlow, MoonError> {
        handle_flow!(self.local_cache.on_emit(&event).await);

        Ok(EventFlow::Continue)
    }
}
