#![allow(unused_assignments)]

use miette::Diagnostic;
use moon_common::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ActionGraphError {
    #[diagnostic(code(action_graph::cycle_detected))]
    #[error("A dependency cycle has been detected for {}.", .0.style(Style::Label))]
    CycleDetected(String),

    #[diagnostic(
        code(action_graph::jobs::invalid_index),
        help = "Indexes are zero-based."
    )]
    #[error(
        "An invalid job index was provided. Received {index} but the total number of jobs is {total}."
    )]
    InvalidJobIndex { index: usize, total: usize },

    #[diagnostic(code(action_graph::exec_plan::invalid_jobs))]
    #[error(
        "An execution plan was provided with task targets partitioned across jobs, but the pipeline has not been configured for parallelism. Please pass the {} and {} options, or define them in the plan.",
        "--job".style(Style::Shell),
        "--job-total".style(Style::Shell),
    )]
    InvalidPlanJobs,

    #[diagnostic(code(action_graph::missing_toolchain_requirement))]
    #[error(
        "Toolchain {} requires the toolchain {}, but it has not been configured!",
        .id.style(Style::Id),
        .dep_id.style(Style::Id),
    )]
    MissingToolchainRequirement { id: String, dep_id: String },

    #[diagnostic(code(action_graph::exec_plan::mismatched_job_totals))]
    #[error(
        "An execution plan was provided with task targets partitioned across {plan_total} jobs, but the pipeline has been configured for {total} jobs."
    )]
    MismatchedPlanJobTotals { plan_total: usize, total: usize },

    #[diagnostic(code(action_graph::would_cycle))]
    #[error(
        "Unable to create action graph, adding a relationship from action {} to {} would introduce a cycle.",
        .source_action.style(Style::Label),
        .target_action.style(Style::Label),
    )]
    WouldCycle {
        source_action: String,
        target_action: String,
    },
}
