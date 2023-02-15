use moon_action::{ActionNode, ActionStatus};
use moon_cache::get_cache_mode;
use moon_emitter::{Event, EventFlow, Subscriber};
use moon_error::MoonError;
use moon_logger::{color, debug, error, map_list, trace, warn};
use moon_pipeline_provider::get_pipeline_environment;
use moon_platform::Runtime;
use moon_utils::{async_trait, fs};
use moon_workspace::Workspace;
use moonbase::{
    graphql::{
        self, add_job_to_run, create_run, update_job, update_run, AddJobToRun, CreateRun,
        GraphQLQuery, UpdateJob, UpdateRun,
    },
    upload_artifact, ArtifactWriteInput, MoonbaseError,
};
use rustc_hash::FxHashMap;
use tokio::task::JoinHandle;

const LOG_TARGET: &str = "moonbase";

pub struct MoonbaseSubscriber {
    download_urls: FxHashMap<String, Option<String>>,

    // Mapping of actions to job IDs
    job_ids: FxHashMap<String, i64>,

    // Upstream database record ID
    run_id: Option<i64>,

    // In-flight requests
    requests: Vec<JoinHandle<()>>,
}

impl MoonbaseSubscriber {
    pub fn new() -> Self {
        MoonbaseSubscriber {
            download_urls: FxHashMap::default(),
            job_ids: FxHashMap::default(),
            run_id: None,
            requests: vec![],
        }
    }

    async fn update_run(
        &self,
        run_id: &i64,
        auth_token: &str,
        input: update_run::UpdateRunInput,
    ) -> Result<(), MoonError> {
        fn log_failure(id: &i64, message: String) {
            warn!(
                target: LOG_TARGET,
                "Failed to update CI run {}. Failure: {}",
                id,
                color::muted_light(message)
            );
        }

        let Ok(response) = graphql::post_mutation::<update_run::ResponseData>(
            UpdateRun::build_query(update_run::Variables {
                id: *run_id,
                input,
            }),
            Some(auth_token),
        ).await else {
            return Ok(());
        };

        match (response.data, response.errors) {
            (_, Some(errors)) => {
                log_failure(run_id, map_list(&errors, |e| e.message.to_owned()));
            }
            (Some(data), _) => {
                if !data.update_run.user_errors.is_empty() {
                    log_failure(
                        run_id,
                        map_list(&data.update_run.user_errors, |e| e.message.to_owned()),
                    );
                }
            }
            _ => {}
        };

        Ok(())
    }
}

#[async_trait]
impl Subscriber for MoonbaseSubscriber {
    async fn on_emit<'a>(
        &mut self,
        event: &Event<'a>,
        workspace: &Workspace,
    ) -> Result<EventFlow, MoonError> {
        let Some(moonbase) = &workspace.session else {
            return Ok(EventFlow::Continue);
        };

        // CI INSIGHTS

        if moonbase.ci_insights_enabled {
            match event {
                // We must wait for this request to finish before firing off other requests,
                // as we require the run ID from the record saved upstream!
                Event::PipelineStarted {
                    actions_count,
                    context,
                } => {
                    debug!(
                        target: LOG_TARGET,
                        "Pipeline started, attempting to create CI run in moonbase"
                    );

                    fn log_failure(message: String) {
                        error!(
                            target: LOG_TARGET,
                            "Failed to create CI run in moonbase, will not track running jobs. Failure: {}",
                            color::muted_light(message)
                        );
                    }

                    let mut branch = String::new();
                    let mut revision = String::new();
                    let mut request_number = None;

                    if let Some(pipeline_env) = get_pipeline_environment() {
                        branch = pipeline_env.branch;
                        revision = pipeline_env.revision;
                        request_number = pipeline_env.request_id;
                    }

                    if branch.is_empty() {
                        branch = workspace
                            .vcs
                            .get_local_branch()
                            .await
                            .map_err(|e| MoonError::Generic(e.to_string()))?;
                    }

                    if revision.is_empty() {
                        revision = workspace
                            .vcs
                            .get_local_branch_revision()
                            .await
                            .map_err(|e| MoonError::Generic(e.to_string()))?;
                    }

                    let affected_targets = context
                        .primary_targets
                        .iter()
                        .map(|t| t.id.clone())
                        .collect::<Vec<_>>();

                    let touched_files = context
                        .touched_files
                        .iter()
                        .map(|f| {
                            f.strip_prefix(&workspace.root)
                                .unwrap_or(f)
                                .to_string_lossy()
                                .to_string()
                        })
                        .collect::<Vec<_>>();

                    let response = match graphql::post_mutation::<create_run::ResponseData>(
                        CreateRun::build_query(create_run::Variables {
                            input: create_run::CreateRunInput {
                                affected_targets: if affected_targets.is_empty() {
                                    None
                                } else {
                                    Some(affected_targets)
                                },
                                branch,
                                job_count: *actions_count as i64,
                                repository_id: moonbase.repository_id as i64,
                                request_number,
                                revision: Some(revision),
                                touched_files: if touched_files.is_empty() {
                                    None
                                } else {
                                    Some(touched_files)
                                },
                            },
                        }),
                        Some(&moonbase.auth_token),
                    )
                    .await
                    {
                        Ok(res) => res,

                        // If the request fails, dont crash the entire pipeline!
                        Err(error) => {
                            log_failure(error.to_string());

                            return Ok(EventFlow::Continue);
                        }
                    };

                    match (response.data, response.errors) {
                        (_, Some(errors)) => {
                            log_failure(map_list(&errors, |e| e.message.to_owned()));
                        }
                        (Some(data), _) => {
                            if data.create_run.user_errors.is_empty() {
                                let id = data.create_run.run.unwrap().id;

                                debug!(
                                    target: LOG_TARGET,
                                    "CI run created in moonbase (id = {})", id,
                                );

                                self.run_id = Some(id);
                            } else {
                                log_failure(map_list(&data.create_run.user_errors, |e| {
                                    e.message.to_owned()
                                }));
                            }
                        }
                        _ => {}
                    };
                }

                // Update the status and duration when the pipeline finishes!
                Event::PipelineFinished {
                    duration,
                    failed_count,
                    ..
                } => {
                    if let Some(run_id) = &self.run_id {
                        self.update_run(
                            run_id,
                            &moonbase.auth_token,
                            update_run::UpdateRunInput {
                                duration: Some(duration.as_millis() as i64),
                                status: Some(if *failed_count > 0 {
                                    update_run::RunStatus::FAILED
                                } else {
                                    update_run::RunStatus::PASSED
                                }),
                            },
                        )
                        .await?
                    }
                }

                // Update the status when the pipeline aborts!
                Event::PipelineAborted { .. } => {
                    if let Some(run_id) = &self.run_id {
                        self.update_run(
                            run_id,
                            &moonbase.auth_token,
                            update_run::UpdateRunInput {
                                duration: None,
                                status: Some(update_run::RunStatus::ABORTED),
                            },
                        )
                        .await?
                    }
                }

                // Actions map to jobs in moonbase, so create a job record for each action.
                // We also need to wait for these requests so that we can extract the job ID.
                Event::ActionStarted { action, .. } => {
                    fn log_failure(message: String) {
                        warn!(
                            target: LOG_TARGET,
                            "Failed to create job for CI run. Failure: {}",
                            color::muted_light(message)
                        );
                    }

                    if let Some(run_id) = &self.run_id {
                        let Ok(response) = graphql::post_mutation::<add_job_to_run::ResponseData>(
                            AddJobToRun::build_query(add_job_to_run::Variables {
                                input: add_job_to_run::CreateJobInput {
                                    run_id: *run_id,
                                    action: action.label.clone(),
                                    started_at: action
                                        .started_at
                                        .expect("Missing start time for action!"),
                                },
                            }),
                            Some(&moonbase.auth_token),
                        ).await else {
                            return Ok(EventFlow::Continue);
                        };

                        match (response.data, response.errors) {
                            (_, Some(errors)) => {
                                log_failure(map_list(&errors, |e| e.message.to_owned()));
                            }
                            (Some(data), _) => {
                                if data.add_job_to_run.user_errors.is_empty() {
                                    self.job_ids.insert(
                                        action.label.clone(),
                                        data.add_job_to_run.job.unwrap().id,
                                    );
                                } else {
                                    log_failure(map_list(&data.add_job_to_run.user_errors, |e| {
                                        e.message.to_owned()
                                    }));
                                }
                            }
                            _ => {}
                        };
                    }
                }

                // When an action finishes, update the job with the final state!
                Event::ActionFinished { action, .. } => {
                    fn log_failure(message: String) {
                        warn!(
                            target: LOG_TARGET,
                            "Failed to update job for CI run. Failure: {}",
                            color::muted_light(message)
                        );
                    }

                    if let Some(job_id) = self.job_ids.get(&action.label) {
                        let mut input = update_job::UpdateJobInput {
                            attempts: None,
                            duration: action.duration.map(|d| d.as_millis() as i64),
                            finished_at: Some(
                                action.finished_at.expect("Missing finish time for action!"),
                            ),
                            status: Some(map_status(&action.status)),
                        };

                        if let Some(attempts) = &action.attempts {
                            input.attempts = Some(
                                attempts
                                    .iter()
                                    .map(|at| update_job::JobAttemptInput {
                                        duration: at
                                            .duration
                                            .map(|d| d.as_millis() as i64)
                                            .unwrap_or_default(),
                                        finished_at: at
                                            .finished_at
                                            .expect("Missing finish time for attempt!"),
                                        started_at: at.started_at,
                                        status: map_status(&at.status),
                                    })
                                    .collect::<Vec<_>>(),
                            );
                        }

                        let variables = update_job::Variables { id: *job_id, input };
                        let auth_token = moonbase.auth_token.clone();

                        // Run the update in a background thread!
                        self.requests.push(tokio::spawn(async move {
                            if let Ok(response) =
                                graphql::post_mutation::<update_job::ResponseData>(
                                    UpdateJob::build_query(variables),
                                    Some(&auth_token),
                                )
                                .await
                            {
                                match (response.data, response.errors) {
                                    (_, Some(errors)) => {
                                        log_failure(map_list(&errors, |e| e.message.to_owned()));
                                    }
                                    (Some(data), _) => {
                                        if !data.update_job.user_errors.is_empty() {
                                            log_failure(map_list(
                                                &data.update_job.user_errors,
                                                |e| e.message.to_owned(),
                                            ));
                                        }
                                    }
                                    _ => {}
                                };
                            }
                        }));
                    }
                }

                _ => {}
            }
        }

        // REMOTE CACHING

        if moonbase.remote_caching_enabled {
            // We don't want errors to bubble up and crash the program,
            // so instead, we log the error (as a warning) to the console!
            fn log_failure(error: MoonbaseError) {
                warn!(
                    target: LOG_TARGET,
                    "Remote caching failure: {}",
                    error.to_string()
                );
            }

            match event {
                // Check if archive exists in moonbase (the remote) by querying the artifacts endpoint.
                Event::TargetOutputCacheCheck { hash, .. } => {
                    if get_cache_mode().is_readable() {
                        match moonbase.read_artifact(hash).await {
                            Ok(Some((artifact, presigned_url))) => {
                                self.download_urls.insert(artifact.hash, presigned_url);

                                return Ok(EventFlow::Return("remote-cache".into()));
                            }
                            Ok(None) => {
                                // Not remote cached
                            }
                            Err(error) => {
                                log_failure(error);

                                // Fallthrough and check local cache
                            }
                        }
                    }
                }

                // The local cache subscriber uses the `TargetOutputArchiving` event to create
                // the tarball. This runs *after* it's been created so that we can upload it.
                Event::TargetOutputArchived {
                    archive_path,
                    hash,
                    target,
                    ..
                } => {
                    if get_cache_mode().is_writable() && archive_path.exists() {
                        let size = match fs::metadata(archive_path) {
                            Ok(meta) => meta.len(),
                            Err(_) => 0,
                        };

                        // Create the database record
                        match moonbase
                            .write_artifact(
                                hash,
                                ArtifactWriteInput {
                                    target: target.id.to_owned(),
                                    size: size as usize,
                                },
                            )
                            .await
                        {
                            // Upload to cloud storage
                            Ok((_, presigned_url)) => {
                                trace!(
                                    target: LOG_TARGET,
                                    "Uploading artifact {} ({} bytes) to remote cache",
                                    color::file(hash),
                                    if size == 0 {
                                        "unknown".to_owned()
                                    } else {
                                        size.to_string()
                                    }
                                );

                                let hash = (*hash).to_owned();
                                let auth_token = moonbase.auth_token.to_owned();
                                let archive_path = archive_path.to_owned();

                                // Create a fake action label so that we can check the CI cache
                                let action_label =
                                    ActionNode::RunTarget(Runtime::System, target.id.to_owned())
                                        .label();
                                let job_id = self.job_ids.get(&action_label).cloned();

                                // Run this in the background so we don't slow down the pipeline
                                // while waiting for very large archives to upload
                                self.requests.push(tokio::spawn(async move {
                                    if let Err(error) = upload_artifact(
                                        auth_token,
                                        hash,
                                        archive_path,
                                        presigned_url,
                                        job_id,
                                    )
                                    .await
                                    {
                                        log_failure(error);
                                    }
                                }));
                            }
                            Err(error) => {
                                log_failure(error);
                            }
                        }
                    }
                }

                // Attempt to download the artifact from the remote cache to `.moon/outputs/<hash>`.
                // This runs *before* the local cache. So if the download is successful, abort
                // the event flow, otherwise continue and let local cache attempt to hydrate.
                Event::TargetOutputHydrating { hash, .. } => {
                    if get_cache_mode().is_readable() {
                        if let Some(download_url) = self.download_urls.get(*hash) {
                            let archive_file = workspace.cache.get_hash_archive_path(hash);

                            trace!(
                                target: LOG_TARGET,
                                "Downloading artifact {} from remote cache",
                                color::file(hash),
                            );

                            if let Err(error) = moonbase
                                .download_artifact(hash, &archive_file, download_url)
                                .await
                            {
                                log_failure(error);
                            }

                            // Fallthrough to local cache to handle the actual hydration
                        }
                    }
                }

                _ => {}
            }
        }

        // For the last event, we want to ensure that all requests have been completed!
        if event.is_end() {
            for future in self.requests.drain(0..) {
                let _ = future.await;
            }
        }

        Ok(EventFlow::Continue)
    }
}

fn map_status(status: &ActionStatus) -> update_job::JobStatus {
    match status {
        ActionStatus::Cached | ActionStatus::CachedFromRemote => update_job::JobStatus::CACHED,
        ActionStatus::Failed | ActionStatus::FailedAndAbort => update_job::JobStatus::FAILED,
        ActionStatus::Invalid | ActionStatus::Passed => update_job::JobStatus::PASSED,
        ActionStatus::Running => update_job::JobStatus::RUNNING,
        ActionStatus::Skipped => update_job::JobStatus::SKIPPED,
    }
}
