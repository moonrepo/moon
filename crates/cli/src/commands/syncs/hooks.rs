use crate::helpers::create_progress_bar;
use clap::Args;
use moon_actions::{sync_vcs_hooks, unsync_vcs_hooks};
use moon_workspace::Workspace;
use starbase::system;
use starbase_styles::color;

#[derive(Args, Clone, Debug)]
pub struct SyncHooksArgs {
    #[arg(long, help = "Clean and remove previously generated hooks")]
    clean: bool,

    #[arg(long, help = "Bypass cache and force create hooks")]
    force: bool,
}

#[system]
pub async fn sync(args: ArgsRef<SyncHooksArgs>, workspace: ResourceRef<Workspace>) {
    if workspace.config.vcs.hooks.is_empty() {
        println!(
            "No hooks available to sync. Configure them with the {} setting.",
            color::id("vcs.hooks")
        );
        println!(
            "Learn more: {}",
            color::url("https://moonrepo.dev/docs/guides/vcs-hooks")
        );

        return Ok(());
    }

    let done = create_progress_bar(format!("Syncing {} hooks...", workspace.config.vcs.manager));
    let hook_names = workspace
        .config
        .vcs
        .hooks
        .keys()
        .map(color::id)
        .collect::<Vec<_>>()
        .join(", ");

    if args.clean {
        unsync_vcs_hooks(&workspace).await?;

        done(format!("Successfully removed {} hooks", hook_names), true);
    } else {
        sync_vcs_hooks(&workspace, args.force).await?;

        done(format!("Successfully created {} hooks", hook_names), true);
    }
}
