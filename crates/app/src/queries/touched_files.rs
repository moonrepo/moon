use moon_common::is_ci;
use moon_common::path::{standardize_separators, WorkspaceRelativePathBuf};
use moon_vcs::{BoxedVcs, TouchedStatus};
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use starbase::AppResult;
use starbase_styles::color;
use std::env;
use tracing::{debug, trace, warn};

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryTouchedFilesOptions {
    pub base: Option<String>,
    pub default_branch: bool,
    pub head: Option<String>,
    pub json: bool,
    pub local: bool,
    pub status: Vec<TouchedStatus>,
}

#[derive(Default, Deserialize, Serialize)]
pub struct QueryTouchedFilesResult {
    pub files: FxHashSet<WorkspaceRelativePathBuf>,
    pub options: QueryTouchedFilesOptions,
    pub shallow: bool,
}

// If we're in a shallow checkout, many diff commands will fail
macro_rules! check_shallow {
    ($vcs:ident) => {
        if $vcs.is_shallow_checkout().await? {
            warn!("Detected a shallow checkout, unable to run Git commands to determine touched files.");

            if is_ci() {
                warn!("A full Git history is required for affected checks, falling back to an empty files list.");
            } else {
                warn!("A full Git history is required for affected checks, disabling for now.");
            }

            let mut result = QueryTouchedFilesResult::default();
            result.shallow = true;

            return Ok(result);
        }
    };
}

/// Query a list of files that have been modified between branches.
pub async fn query_touched_files(
    vcs: &BoxedVcs,
    options: &QueryTouchedFilesOptions,
) -> AppResult<QueryTouchedFilesResult> {
    debug!("Querying for touched files");

    let default_branch = vcs.get_default_branch().await?;
    let current_branch = vcs.get_local_branch().await?;

    // On default branch, so compare against self -1 revision
    let touched_files_map = if options.default_branch && vcs.is_default_branch(&current_branch) {
        check_shallow!(vcs);

        trace!(
            "On default branch {}, comparing against previous revision",
            current_branch
        );

        vcs.get_touched_files_against_previous_revision(&default_branch)
            .await?

        // On a branch, so compare branch against remote base/default branch
    } else if !options.local {
        check_shallow!(vcs);

        let base = env::var("MOON_BASE").unwrap_or_else(|_| {
            options
                .base
                .as_deref()
                .unwrap_or(&default_branch)
                .to_owned()
        });

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
            options
                .status
                .iter()
                .map(|f| color::symbol(f.to_string()))
                .collect::<Vec<_>>()
                .join(", ")
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
        .map(|f| WorkspaceRelativePathBuf::from(standardize_separators(f)))
        .collect();

    if !touched_files.is_empty() {
        debug!(
            files = ?touched_files.iter().map(|f| f.as_str()).collect::<Vec<_>>(),
            "Found touched files",
        );
    }

    Ok(QueryTouchedFilesResult {
        files: touched_files,
        options: options.to_owned(),
        shallow: false,
    })
}
