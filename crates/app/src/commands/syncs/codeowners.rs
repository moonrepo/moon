use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::element;
use moon_actions::operations::{sync_codeowners, unsync_codeowners};
use moon_console::ui::{Container, Notice, StyledText, Variant};
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct SyncCodeownersArgs {
    #[arg(long, help = "Clean and remove previously generated file")]
    clean: bool,

    #[arg(long, help = "Bypass cache and force create file")]
    force: bool,
}

#[instrument(skip_all)]
pub async fn sync(session: MoonSession, args: SyncCodeownersArgs) -> AppResult {
    let context = session.get_app_context().await?;

    let message = if args.clean {
        let codeowners_path = unsync_codeowners(&context).await?;

        format!("Removed <path>{}</path>", codeowners_path.display())
    } else {
        let workspace_graph = session.get_workspace_graph().await?;
        let codeowners_path = sync_codeowners(&context, &workspace_graph, args.force).await?;

        if let Some(path) = codeowners_path {
            format!("Synced codeowners to <path>{}</path>", path.display())
        } else {
            "Synced codeowners".into()
        }
    };

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(content: message)
            }
        }
    })?;

    Ok(None)
}
