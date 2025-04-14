use crate::session::MoonSession;
use starbase::AppResult;

pub async fn debug_vcs(session: MoonSession) -> AppResult {
    let vcs = session.get_vcs_adapter()?;

    println!("config");
    dbg!(&session.workspace_config.vcs);

    println!("vcs");
    dbg!(&vcs);

    println!("default_branch = {}", vcs.get_default_branch().await?);
    println!(
        "default_branch_revision = {}",
        vcs.get_default_branch_revision().await?
    );
    println!("local_branch = {}", vcs.get_local_branch().await?);
    println!(
        "local_branch_revision = {}",
        vcs.get_local_branch_revision().await?
    );

    println!("touched_files");
    dbg!(vcs.get_touched_files().await?);

    println!("touched_files_against_previous_revision");
    dbg!(
        vcs.get_touched_files_against_previous_revision("HEAD")
            .await?
    );

    Ok(None)
}
