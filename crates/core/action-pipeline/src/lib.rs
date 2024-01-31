mod actions;
mod errors;
pub mod estimator;
mod pipeline;
mod processor;
mod run_report;
mod subscribers;

pub use errors::*;
pub use moon_action_context::*;
pub use pipeline::*;

pub(crate) fn label_to_the_moon() -> String {
    use starbase_styles::color::paint;

    [
        paint(55, "❯"),
        paint(56, "❯❯"),
        paint(57, "❯ t"),
        paint(63, "o t"),
        paint(69, "he "),
        paint(75, "mo"),
        paint(81, "on"),
    ]
    .iter()
    .map(|i| i.to_string())
    .collect::<Vec<_>>()
    .join("")
}
