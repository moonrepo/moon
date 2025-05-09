use crate::session::MoonSession;
use starbase::AppResult;

pub async fn debug_config(session: MoonSession) -> AppResult {
    dbg!(&session.workspace_config);

    dbg!(&session.toolchain_config);

    dbg!(&session.tasks_config);

    Ok(None)
}
