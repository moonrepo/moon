use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::element;
use moon_actions::operations::{sync_vcs_hooks, unsync_vcs_hooks};
use moon_console::ui::{Container, Notice, StyledText, Variant};
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct SyncHooksArgs {
    #[arg(long, help = "Clean and remove previously generated hooks")]
    clean: bool,

    #[arg(long, help = "Bypass cache and force create hooks")]
    force: bool,
}

#[instrument(skip_all)]
pub async fn sync(session: MoonSession, args: SyncHooksArgs) -> AppResult {
    if session.workspace_config.vcs.hooks.is_empty() {
        session.console.render(element! {
            Container {
                Notice(variant: Variant::Caution) {
                    StyledText(
                        content: "No hooks available to sync. Configure them with the <property>vcs.hooks</property> setting.",
                    )
                    StyledText(
                        content: "Learn more: <url>https://moonrepo.dev/docs/guides/vcs-hooks</url>"
                    )
                }
            }
        })?;

        return Ok(None);
    }

    let context = session.get_app_context().await?;
    let hook_names = session
        .workspace_config
        .vcs
        .hooks
        .keys()
        .map(|name| format!("<id>{name}</id>"))
        .collect::<Vec<_>>()
        .join(", ");

    let message = if args.clean {
        unsync_vcs_hooks(&context).await?;

        format!("Removed {hook_names} hooks")
    } else {
        sync_vcs_hooks(&context, args.force).await?;

        format!("Created {hook_names} hooks")
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
