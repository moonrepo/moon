use crate::session::MoonSession;
use starbase::AppResult;

pub async fn debug_config(session: MoonSession) -> AppResult {
    dbg!(&session.moon_env);

    dbg!(&session.proto_env);

    dbg!(session.proto_env.load_config()?);

    dbg!(&session.workspace_config);

    dbg!(&session.extensions_config);

    // TODO: fix the system toolchain data being too large to print
    dbg!(&session.toolchains_config);

    dbg!(&session.tasks_config);

    Ok(None)
}
