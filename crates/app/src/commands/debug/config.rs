use crate::session::{MoonSession, SessionResult};

pub async fn debug_config(session: MoonSession) -> SessionResult {
    dbg!(&session.moon_env);

    dbg!(&session.proto_env);

    dbg!(session.proto_env.load_config()?);

    dbg!(&session.workspace_config);

    dbg!(&session.extensions_config);

    dbg!(&session.toolchains_config);

    dbg!(&session.tasks_config);

    Ok(None)
}
