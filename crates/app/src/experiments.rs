use crate::{CliSession, Commands};
use moon_action::Action;
use moon_action_context::ActionContext;
use moon_action_graph::ActionGraph;
use moon_action_pipeline::Pipeline as ActionPipeline;
use moon_action_pipeline2::ActionPipeline as ExpActionPipeline;

pub async fn run_action_pipeline(
    session: &CliSession,
    action_graph: ActionGraph,
    action_context: Option<ActionContext>,
) -> miette::Result<Vec<Action>> {
    // v2
    if session.workspace_config.experiments.action_pipeline_v2 {
        let mut pipeline = ExpActionPipeline::new(
            session.get_app_context()?,
            session.get_project_graph().await?,
        );

        if let Some(concurrency) = &session.cli.concurrency {
            pipeline.concurrency = Some(*concurrency);
        }

        match &session.cli.command {
            Commands::Check(cmd) => {
                pipeline.summarize = cmd.summary;
                pipeline.bail = true;
            }
            Commands::Ci(_) => {
                pipeline.summarize = true;
            }
            Commands::Run(cmd) => {
                pipeline.summarize = cmd.summary;
                pipeline.bail = true;
            }
            _ => {}
        };

        let results = match action_context {
            Some(ctx) => pipeline.run_with_context(action_graph, ctx).await?,
            None => pipeline.run(action_graph).await?,
        };

        return Ok(results);
    }

    // v1
    {
        let mut pipeline = ActionPipeline::new(
            session.get_app_context()?,
            session.get_project_graph().await?,
        );

        if let Some(concurrency) = &session.cli.concurrency {
            pipeline.concurrency(*concurrency);
        }

        match &session.cli.command {
            Commands::Check(cmd) => {
                pipeline
                    .summarize(cmd.summary)
                    .generate_report("runReport.json")
                    .bail_on_error();
            }
            Commands::Ci(_) => {
                pipeline.summarize(true).generate_report("ciReport.json");
            }
            Commands::Run(cmd) => {
                pipeline
                    .summarize(cmd.summary)
                    .generate_report("runReport.json")
                    .bail_on_error();
            }
            _ => {}
        };

        let results = pipeline.run(action_graph, action_context).await?;

        Ok(results)
    }
}
