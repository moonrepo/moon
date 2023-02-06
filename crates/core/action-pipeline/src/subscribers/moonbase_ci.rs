use moon_action::ActionStatus;
use moon_emitter::{Event, EventFlow, Subscriber};
use moon_error::MoonError;
use moon_logger::{color, debug, error, map_list, warn};
use moon_utils::async_trait;
use moon_workspace::Workspace;
use moonbase::graphql::{
    self, add_job_to_run, create_run, update_job, AddJobToRun, CreateRun, GraphQLQuery, UpdateJob,
};
use rustc_hash::FxHashMap;
use tokio::task::JoinHandle;

const LOG_TARGET: &str = "moonbase:ci-insights";

pub struct MoonbaseCiSubscriber {
    // Mapping of actions to job IDs
    job_ids: FxHashMap<String, i64>,

    // Upstream database record ID
    run_id: Option<i64>,

    // In-flight requests
    requests: Vec<JoinHandle<()>>,
}

impl MoonbaseCiSubscriber {
    pub fn new() -> Self {
        MoonbaseCiSubscriber {
            job_ids: FxHashMap::default(),
            run_id: None,
            requests: vec![],
        }
    }

    pub fn not_enabled() {
        debug!(
            target: LOG_TARGET,
            "A moonbase session exists but CI insights is not enabled. Will not track CI runs!"
        );
    }
}

#[async_trait]
impl Subscriber for MoonbaseCiSubscriber {
    async fn on_emit<'a>(
        &mut self,
        event: &Event<'a>,
        workspace: &Workspace,
    ) -> Result<EventFlow, MoonError> {
        let Some(moonbase) = &workspace.session else {
            return Ok(EventFlow::Continue);
        };

        match event {
            // We must wait for this request to finish before firing off other requests,
            // as we require the run ID from the record saved upstream!
            Event::PipelineStarted { actions_count } => {
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

                let branch = workspace
                    .vcs
                    .get_local_branch()
                    .await
                    .map_err(|e| MoonError::Generic(e.to_string()))?;

                let response = match graphql::post_mutation::<create_run::ResponseData>(
                    CreateRun::build_query(create_run::Variables {
                        input: create_run::CreateRunInput {
                            branch,
                            job_count: *actions_count as i64,
                            repository_id: moonbase.repository_id as i64,
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

            // TODO
            Event::PipelineFinished { .. } => {}

            // TODO
            Event::PipelineAborted { .. } => {}

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
                        if let Ok(response) = graphql::post_mutation::<update_job::ResponseData>(
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
                                        log_failure(map_list(&data.update_job.user_errors, |e| {
                                            e.message.to_owned()
                                        }));
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
