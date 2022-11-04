use async_trait::async_trait;
use moon_workspace::Workspace;

use super::dto::StatusDto;

#[async_trait]
pub trait ServiceTrait: Sync + Send {
    fn status(&self) -> StatusDto;
}

pub struct Service {
    pub workspace: Workspace,
}

impl Service {
    pub fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl ServiceTrait for Service {
    fn status(&self) -> StatusDto {
        StatusDto { is_running: true }
    }
}
