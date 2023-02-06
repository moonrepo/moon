use moon_emitter::{Event, EventFlow, Subscriber};
use moon_error::MoonError;
use moon_logger::{color, debug, error, trace};
use moon_utils::async_trait;
use moon_workspace::Workspace;
use moonbase::graphql::{self, CreateRunInput, CreateRunPayload};
use moonbase::Response;
use tokio::task::JoinHandle;

const LOG_TARGET: &str = "moonbase:ci-insights";

pub struct MoonbaseCiSubscriber {
    // Upstream database record
    run_id: Option<i32>,

    // In-flight requests
    requests: Vec<JoinHandle<()>>,
}

impl MoonbaseCiSubscriber {
    pub fn new() -> Self {
        MoonbaseCiSubscriber {
            run_id: None,
            requests: vec![],
        }
    }

    pub fn not_enabled() {
        debug!(
            target: LOG_TARGET,
            "A moonbase session exists, but CI insights has been disaled. Will not track CI runs!"
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
                let branch = workspace
                    .vcs
                    .get_local_branch()
                    .await
                    .map_err(|e| MoonError::Generic(e.to_string()))?;

                let response: Response<CreateRunPayload> = graphql::post_mutation(
                    r#"mutation CreateRun($input: CreateRunInput!) {
  createRun(input: $input) {
    run {
      id
    }
    userErrors {
      message
    }
  }
}"#,
                    CreateRunInput {
                        branch,
                        job_count: *actions_count,
                        repository_id: moonbase.repository_id,
                    },
                    Some(&moonbase.auth_token),
                )
                .await
                .map_err(|e| MoonError::Generic(e.to_string()))?;

                // Handle all the possible failure states!
                match response {
                    Response::Failure { message, .. } => {
                        error!(
                            target: LOG_TARGET,
                            "Failed to create run in moonbase, will not track running jobs. {}",
                            message
                        );
                    }
                    Response::Success(res) => {
                        dbg!(res);
                    }
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
