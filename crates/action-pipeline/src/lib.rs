// Auto-trait (`Send`) proofs for the deeply nested job futures spawned in
// `ActionPipeline` exceed the default trait solver recursion limit of 128
#![recursion_limit = "256"]

mod action_pipeline;
mod action_runner;
mod event_emitter;
mod job;
mod job_context;
mod job_dispatcher;
pub mod reports;
mod subscribers;

pub use action_pipeline::*;
