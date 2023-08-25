use crate::enums::TouchedStatus;
use crate::helpers::map_list;
use moon_common::path::{standardize_separators, WorkspaceRelativePathBuf};
use moon_workspace::Workspace;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use starbase::AppResult;
use starbase_styles::color;
use std::env;
use tracing::{debug, trace};

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryTouchedFilesOptions {
    pub base: Option<String>,
    pub default_branch: bool,
    pub head: Option<String>,
    pub json: bool,
    pub local: bool,
    #[serde(skip)]
    pub log: bool,
    pub status: Vec<TouchedStatus>,
}

#[derive(Deserialize, Serialize)]
pub struct QueryTouchedFilesResult {
    pub files: FxHashSet<WorkspaceRelativePathBuf>,
    pub options: QueryTouchedFilesOptions,
}

/// Query a list of files that have been modified between branches.
pub async fn query_touched_files(
    workspace: &Workspace,
    options: &QueryTouchedFilesOptions,
) -> AppResult<FxHashSet<WorkspaceRelativePathBuf>> {
    debug!("Querying for touched files");

    let vcs = &workspace.vcs;
    let default_branch = vcs.get_default_branch().await?;
    let current_branch = vcs.get_local_branch().await?;

    // On default branch, so compare against self -1 revision
    let touched_files_map = if options.default_branch && vcs.is_default_branch(current_branch) {
        trace!(
            "On default branch {}, comparing against previous revision",
            current_branch
        );

        vcs.get_touched_files_against_previous_revision(default_branch)
            .await?

        // On a branch, so compare branch against remote base/default branch
    } else if !options.local {
        let base = env::var("MOON_BASE")
            .unwrap_or_else(|_| options.base.as_deref().unwrap_or(default_branch).to_owned());

        let head = env::var("MOON_HEAD")
            .unwrap_or_else(|_| options.head.as_deref().unwrap_or("HEAD").to_owned());

        trace!(
            "Against remote using base \"{}\" with head \"{}\"",
            base,
            head,
        );

        vcs.get_touched_files_between_revisions(&base, &head)
            .await?

        // Otherwise, check locally touched files
    } else {
        trace!("Against locally touched");

        vcs.get_touched_files().await?
    };

    let mut touched_files_to_log = vec![];
    let mut touched_files = FxHashSet::default();

    if options.status.is_empty() {
        debug!(
            "Filtering based on touched status \"{}\"",
            color::symbol(TouchedStatus::All.to_string())
        );

        touched_files.extend(touched_files_map.all());
    } else {
        debug!(
            "Filtering based on touched status \"{}\"",
            map_list(&options.status, |f| color::symbol(f.to_string()))
        );

        for status in &options.status {
            touched_files.extend(match status {
                TouchedStatus::Added => touched_files_map.added.iter().collect(),
                TouchedStatus::All => touched_files_map.all(),
                TouchedStatus::Deleted => touched_files_map.deleted.iter().collect(),
                TouchedStatus::Modified => touched_files_map.modified.iter().collect(),
                TouchedStatus::Staged => touched_files_map.staged.iter().collect(),
                TouchedStatus::Unstaged => touched_files_map.unstaged.iter().collect(),
                TouchedStatus::Untracked => touched_files_map.untracked.iter().collect(),
            });
        }
    }

    let touched_files: FxHashSet<WorkspaceRelativePathBuf> = touched_files
        .iter()
        .map(|f| {
            if options.log {
                touched_files_to_log.push(format!("  {}", color::file(f)));
            }

            WorkspaceRelativePathBuf::from(standardize_separators(f))
        })
        .collect();

    if !touched_files_to_log.is_empty() {
        touched_files_to_log.sort();

        if options.log {
            println!("{}", touched_files_to_log.join("\n"));
        } else {
            debug!("Found touched files:\n{}", touched_files_to_log.join("\n"));
        }
    }

    Ok(touched_files)
}
