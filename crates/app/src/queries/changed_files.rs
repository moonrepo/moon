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
// use std::time::Duration;
// use tokio::io::AsyncReadExt;
// use tokio::time::timeout;
use tracing::{debug, trace, warn};

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryChangedFilesOptions {
    pub base: Option<String>,
    pub default_branch: bool,
    pub head: Option<String>,
    pub json: bool,
    pub local: bool,
    pub status: Vec<ChangedStatus>,
    pub stdin: bool,
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

/// Query a list of files that have been modified between branches.
pub async fn query_changed_files(
    vcs: &BoxedVcs,
    options: &QueryChangedFilesOptions,
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
                .map(|f| color::symbol(f.to_string()))
                .collect::<Vec<_>>()
                .join(", ")
        );

        for status in &options.status {
            changed_files.extend(changed_files_map.select(*status));
        }
    }

    let changed_files: FxHashSet<WorkspaceRelativePathBuf> = changed_files
        .iter()
        .map(|f| WorkspaceRelativePathBuf::from(standardize_separators(f)))
        .collect();

    debug!(
        files = ?changed_files.iter().map(|f| f.as_str()).collect::<Vec<_>>(),
        "Found changed files",
    );

    Ok(QueryChangedFilesResult {
        files: changed_files,
        options: options.to_owned(),
        shallow: false,
    })
}

pub async fn query_changed_files_with_stdin(
    vcs: &BoxedVcs,
    options: &QueryChangedFilesOptions,
) -> miette::Result<QueryChangedFilesResult> {
    debug!("Querying for changed files");

    if !options.stdin {
        return query_changed_files(vcs, options).await;
    }

    let mut buffer = String::new();

    // Only read piped data when stdin is not a TTY,
    // otherwise the process will hang indefinitely waiting for EOF.
    if !stdin().is_terminal() {
        // if let Ok(read_result) = timeout(
        //     Duration::from_secs(10),
        //     tokio::io::stdin().read_to_string(&mut buffer),
        // )
        // .await
        // {
        //     read_result.into_diagnostic()?;
        // }

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

    query_changed_files(vcs, options).await
}

pub async fn load_changed_files(
    vcs: &BoxedVcs,
) -> miette::Result<FxHashSet<WorkspaceRelativePathBuf>> {
    let ci = is_ci();

    query_changed_files_with_stdin(
        vcs,
        &QueryChangedFilesOptions {
            default_branch: ci,
            local: !ci,
            stdin: true,
            ..QueryChangedFilesOptions::default()
        },
    )
    .await
    .map(|result| result.files)
}
