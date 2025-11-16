use crate::app_options::AffectedOption;
use miette::IntoDiagnostic;
use moon_common::is_ci;
use moon_common::path::{WorkspaceRelativePathBuf, standardize_separators};
use moon_env_var::GlobalEnvBag;
use moon_vcs::{BoxedVcs, ChangedStatus};
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use starbase_styles::color;
use starbase_utils::json;
use std::io::{IsTerminal, Read, stdin};
use tracing::{debug, trace, warn};

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryChangedFilesOptions {
    pub base: Option<String>,
    pub default_branch: bool,
    pub head: Option<String>,
    pub local: bool,
    pub status: Vec<ChangedStatus>,
    pub stdin: bool,
}

impl QueryChangedFilesOptions {
    pub fn apply_affected(&mut self, by: &AffectedOption) {
        let local = by.is_local();

        if self.base.is_none() {
            self.base = by.get_base();
        }

        if self.head.is_none() {
            self.head = by.get_head();
        }

        self.default_branch = !local;
        self.local = local;
    }
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub struct QueryChangedFilesResult {
    pub files: FxHashSet<WorkspaceRelativePathBuf>,
    pub options: QueryChangedFilesOptions,
    pub shallow: bool,
}

// If we're in a shallow checkout, many diff commands will fail
macro_rules! check_shallow {
    ($vcs:ident) => {
        if $vcs.is_shallow_checkout().await? {
            warn!("Detected a shallow checkout, unable to run Git commands to determine changed files.");

            if is_ci() {
                warn!("A full Git history is required for affected checks, falling back to an empty files list.");
            } else {
                warn!("A full Git history is required for affected checks, disabling for now.");
            }

            let mut result = QueryChangedFilesResult::default();
            result.shallow = true;

            return Ok(result);
        }
    };
}

pub async fn query_changed_files(
    vcs: &BoxedVcs,
    options: QueryChangedFilesOptions,
) -> miette::Result<QueryChangedFilesResult> {
    debug!("Querying for changed files");

    if options.stdin {
        query_changed_files_with_stdin(vcs, options).await
    } else {
        query_changed_files_without_stdin(vcs, options).await
    }
}

async fn query_changed_files_without_stdin(
    vcs: &BoxedVcs,
    options: QueryChangedFilesOptions,
) -> miette::Result<QueryChangedFilesResult> {
    let bag = GlobalEnvBag::instance();
    let default_branch = vcs.get_default_branch().await?;
    let current_branch = vcs.get_local_branch().await?;
    let base_value = bag.get("MOON_BASE").or(options.base.clone());
    let base = base_value.as_deref().unwrap_or(&default_branch);
    let head_value = bag.get("MOON_HEAD").or(options.head.clone());
    let head = head_value.as_deref().unwrap_or("HEAD");

    // Determine whether we should check against the previous
    // commit using a HEAD~1 query
    let check_against_previous = base_value.is_none()
        && head_value.is_none()
        && vcs.is_default_branch(&current_branch)
        && options.default_branch;

    // Don't check for shallow if base is set,
    // since we can assume the user knows what they're doing
    if base_value.is_none() {
        check_shallow!(vcs);
    }

    // Check locally changed files
    let changed_files_map = if options.local && base_value.is_none() {
        trace!("Against local index");

        vcs.get_changed_files().await?
    }
    // Otherwise compare against previous commit
    else if check_against_previous {
        trace!(
            "Against previous revision, as we're on the default branch \"{}\"",
            current_branch
        );

        vcs.get_changed_files_against_previous_revision(&default_branch)
            .await?
    }
    // Otherwise against remote between 2 revisions
    else {
        trace!(
            "Against remote using base \"{}\" with head \"{}\"",
            base, head,
        );

        vcs.get_changed_files_between_revisions(base, head).await?
    };

    let mut changed_files = FxHashSet::default();

    if options.status.is_empty() {
        debug!(
            "Filtering based on changed status {}",
            color::symbol(ChangedStatus::All.to_string())
        );

        changed_files.extend(changed_files_map.all());
    } else {
        debug!(
            "Filtering based on changed status {}",
            options
                .status
                .iter()
                .map(|status| color::symbol(status.to_string()))
                .collect::<Vec<_>>()
                .join(", ")
        );

        for status in &options.status {
            changed_files.extend(changed_files_map.select(*status));
        }
    }

    let changed_files: FxHashSet<WorkspaceRelativePathBuf> = changed_files
        .iter()
        .map(|file| WorkspaceRelativePathBuf::from(standardize_separators(file)))
        .collect();

    debug!(
        files = ?changed_files.iter().map(|file| file.as_str()).collect::<Vec<_>>(),
        "Found changed files",
    );

    Ok(QueryChangedFilesResult {
        files: changed_files,
        options,
        shallow: false,
    })
}

async fn query_changed_files_with_stdin(
    vcs: &BoxedVcs,
    options: QueryChangedFilesOptions,
) -> miette::Result<QueryChangedFilesResult> {
    let mut buffer = String::new();

    // Only read piped data when stdin is not a TTY,
    // otherwise the process will hang indefinitely waiting for EOF.
    if !stdin().is_terminal() {
        stdin().read_to_string(&mut buffer).into_diagnostic()?;
    }

    // If piped via stdin, parse and use it
    if !buffer.is_empty() {
        // As JSON
        if buffer.starts_with('{') {
            debug!("Received from stdin as JSON");

            let result: QueryChangedFilesResult = json::parse(&buffer)?;

            return Ok(result);
        }
        // As lines
        else {
            debug!("Received from stdin as separate lines");

            let files =
                FxHashSet::from_iter(buffer.split('\n').map(WorkspaceRelativePathBuf::from));

            return Ok(QueryChangedFilesResult {
                files,
                ..Default::default()
            });
        }
    }

    query_changed_files_without_stdin(vcs, options).await
}

pub async fn query_changed_files_for_affected(
    vcs: &BoxedVcs,
    by: Option<&AffectedOption>,
) -> miette::Result<FxHashSet<WorkspaceRelativePathBuf>> {
    let ci = is_ci();
    let mut options = QueryChangedFilesOptions {
        default_branch: ci,
        local: !ci,
        stdin: true,
        ..Default::default()
    };

    if let Some(by) = by {
        options.apply_affected(by);
    }

    query_changed_files(vcs, options)
        .await
        .map(|result| result.files)
}
