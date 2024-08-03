mod from_package_json;
mod from_turborepo;

pub use from_package_json::{from_package_json, FromPackageJsonArgs};
pub use from_turborepo::*;

use crate::session::CliSession;
use clap::Subcommand;
use miette::miette;
use starbase::AppResult;
use starbase_styles::color;

#[derive(Clone, Debug, Subcommand)]
pub enum MigrateCommands {
    #[command(
        name = "from-package-json",
        about = "Migrate `package.json` scripts and dependencies to `moon.*`."
    )]
    FromPackageJson(FromPackageJsonArgs),

    #[command(
        name = "from-turborepo",
        about = "Migrate `turbo.json` to moon configuration files."
    )]
    FromTurborepo,
}

pub async fn check_dirty_repo(session: &CliSession) -> AppResult {
    if !session
        .get_vcs_adapter()?
        .get_touched_files()
        .await?
        .all()
        .is_empty()
    {
        return Err(miette!(
            code = "moon::migrate",
            "Commit or stash your changes before running this command, or use the {} flag to disable this check.",
            color::property("--skipTouchedFilesCheck"),
        ));
    }

    Ok(())
}
