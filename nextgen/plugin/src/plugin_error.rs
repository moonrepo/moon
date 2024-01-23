use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use thiserror::Error;
use warpgate::Id;

#[derive(Error, Debug, Diagnostic)]
pub enum PluginError {
    #[diagnostic(code(plugin::existing_id), help = "Try another identifier?")]
    #[error("The {} plugin {} already exists. Overriding the plugin is not supported.", .name, .id.style(Style::Id))]
    ExistingId { name: String, id: Id },

    #[diagnostic(
        code(plugin::unknown_id),
        help = "Has it been configured or registered?"
    )]
    #[error("The {} plugin {} does not exist.", .name, .id.style(Style::Id))]
    UnknownId { name: String, id: Id },
}
