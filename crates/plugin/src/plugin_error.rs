use crate::plugin::PluginType;
use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum PluginError {
    #[diagnostic(code(plugin::existing_id), help = "Try another identifier?")]
    #[error(
        "The {} plugin {} already exists. Overriding the plugin is not supported.",
        .ty.get_label(),
        .id.style(Style::Id),
    )]
    ExistingId { id: String, ty: PluginType },

    #[diagnostic(
        code(plugin::unknown_id),
        help = "Has it been configured or registered?"
    )]
    #[error("The {} plugin {} does not exist.", .ty.get_label(), .id.style(Style::Id))]
    UnknownId { id: String, ty: PluginType },
}
