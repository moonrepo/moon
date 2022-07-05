use crate::enums::TouchedStatus;
use moon_logger::{color, debug, trace};
use moon_project::TouchedFilePaths;
use moon_utils::path;
use moon_workspace::{Workspace, WorkspaceError};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

const TARGET: &str = "moon:query:touched-files";

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryTouchedFilesOptions {
    pub base: String,
    pub default_branch: bool,
    pub head: String,
    pub local: bool,
    #[serde(skip)]
    pub log: bool,
    pub status: TouchedStatus,
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
    debug!(target: TARGET, "Querying for touched files");

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
            target: TARGET,
            "On default branch {}, comparing against previous revision",
            current_branch
        );

        vcs.get_touched_files_against_previous_revision(default_branch)
            .await?

        // On a branch, so compare branch against upstream base/default branch
    } else if !options.local {
        trace!(
            target: TARGET,
            "Against upstream using base \"{}\" with head \"{}\"",
            options.base,
            options.head,
        );

        vcs.get_touched_files_between_revisions(&options.base, &options.head)
            .await?

        // Otherwise, check locally touched files
    } else {
        trace!(target: TARGET, "Against locally touched",);

        vcs.get_touched_files().await?
    };

    let mut touched_files_to_log = vec![];

    debug!(
        target: TARGET,
        "Filtering based on touched status \"{}\"", options.status
    );

    let touched_files = match options.status {
        TouchedStatus::Added => touched_files_map.added,
        TouchedStatus::All => touched_files_map.all,
        TouchedStatus::Deleted => touched_files_map.deleted,
        TouchedStatus::Modified => touched_files_map.modified,
        TouchedStatus::Staged => touched_files_map.staged,
        TouchedStatus::Unstaged => touched_files_map.unstaged,
        TouchedStatus::Untracked => touched_files_map.untracked,
    };

    let touched_files: HashSet<PathBuf> = touched_files
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
