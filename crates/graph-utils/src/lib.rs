// mod dag_partition;
mod graph_context;
mod graph_formats;
mod graph_traits;

// pub use dag_partition::*;
pub use graph_context::*;
pub use graph_formats::*;
pub use graph_traits::*;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum NodeState<T> {
    Loading,
    Loaded(T),
}
