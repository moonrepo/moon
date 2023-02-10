use crate::enums::TouchedStatus;
use moon_logger::{color, debug, map_list, trace};
use moon_task::TouchedFilePaths;
use moon_utils::path;
use moon_workspace::{Workspace, WorkspaceError};
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const LOG_TARGET: &str = "moon:query:touched-files";

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryTouchedFilesOptions {
    pub base: String,
    pub default_branch: bool,
    pub head: String,
    pub json: bool,
    pub local: bool,
    #[serde(skip)]
    pub log: bool,
    pub status: Vec<TouchedStatus>,
}

#[derive(Deserialize, Serialize)]
pub struct QueryTouchedFilesResult {
    pub files: TouchedFilePaths,
    pub options: QueryTouchedFilesOptions,
}

/// Query a list of files that have been modified between branches.
pub async fn query_touched_files(
    workspace: &Workspace,
    options: &mut QueryTouchedFilesOptions,
) -> Result<TouchedFilePaths, WorkspaceError> {
    debug!(target: LOG_TARGET, "Querying for touched files");

    let vcs = &workspace.vcs;
    let default_branch = vcs.get_default_branch();
    let current_branch = vcs.get_local_branch().await?;

    if options.base.is_empty() {
        options.base = default_branch.to_owned();
    }

    if options.head.is_empty() {
        options.head = "HEAD".to_string();
    }

    // On default branch, so compare against self -1 revision
    let touched_files_map = if options.default_branch && vcs.is_default_branch(&current_branch) {
        trace!(
            target: LOG_TARGET,
            "On default branch {}, comparing against previous revision",
            current_branch
        );

        vcs.get_touched_files_against_previous_revision(default_branch)
            .await?

        // On a branch, so compare branch against remote base/default branch
    } else if !options.local {
        trace!(
            target: LOG_TARGET,
            "Against remote using base \"{}\" with head \"{}\"",
            options.base,
            options.head,
        );

        vcs.get_touched_files_between_revisions(&options.base, &options.head)
            .await?

        // Otherwise, check locally touched files
    } else {
        trace!(target: LOG_TARGET, "Against locally touched",);

        vcs.get_touched_files().await?
    };

    let mut touched_files_to_log = vec![];
    let mut touched_files = FxHashSet::default();

    if options.status.is_empty() {
        debug!(
            target: LOG_TARGET,
            "Filtering based on touched status \"{}\"",
            color::symbol(TouchedStatus::All.to_string())
        );

        touched_files.extend(&touched_files_map.all);
    } else {
        debug!(
            target: LOG_TARGET,
            "Filtering based on touched status \"{}\"",
            map_list(&options.status, |f| color::symbol(f.to_string()))
        );

        for status in &options.status {
            touched_files.extend(match status {
                TouchedStatus::Added => &touched_files_map.added,
                TouchedStatus::All => &touched_files_map.all,
                TouchedStatus::Deleted => &touched_files_map.deleted,
                TouchedStatus::Modified => &touched_files_map.modified,
                TouchedStatus::Staged => &touched_files_map.staged,
                TouchedStatus::Unstaged => &touched_files_map.unstaged,
                TouchedStatus::Untracked => &touched_files_map.untracked,
            });
        }
    }

    let touched_files: FxHashSet<PathBuf> = touched_files
        .iter()
        .map(|f| {
            if options.log {
                touched_files_to_log.push(format!("  {}", color::file(f)));
            }

            workspace.root.join(path::normalize_separators(f))
        })
        .collect();

    if options.log {
        touched_files_to_log.sort();

        println!("{}", touched_files_to_log.join("\n"));
    }

    Ok(touched_files)
}
