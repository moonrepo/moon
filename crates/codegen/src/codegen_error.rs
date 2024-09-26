use miette::Diagnostic;
use moon_common::{Id, Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum CodegenError {
    #[diagnostic(code(codegen::template::fs_only))]
    #[error(
        "Unable to create a new template, as the destination must be a local file system path.\nPlease add a file path to the {} setting.",
        "generator.templates".style(Style::Property),
    )]
    CreateFileSystemOnly,

    #[diagnostic(code(codegen::template::exists))]
    #[error(
        "A template with the name {} already exists at {}.",
        .0.style(Style::Id),
        .1.style(Style::Path),
    )]
    ExistingTemplate(Id, PathBuf),

    #[diagnostic(code(codegen::args::parse_failed))]
    #[error("Failed to parse variables from arguments.")]
    FailedToParseArgs {
        #[diagnostic_source]
        error: miette::Report,
    },

    #[diagnostic(code(codegen::template::missing))]
    #[error(
        "No template with the name {} could be found at any of the configured template locations.",
        .0.style(Style::Id),
    )]
    MissingTemplate(Id),

    #[diagnostic(code(codegen::template::duplicate))]
    #[error(
        "Found multiple templates with the same name {}.\nOriginal template at {}.\nCurrent template at {}.",
        .id.style(Style::Id),
        .original.style(Style::Path),
        .current.style(Style::Path),
    )]
    DuplicateTemplate {
        id: Id,
        original: PathBuf,
        current: PathBuf,
    },

    #[diagnostic(code(codegen::template_file::load_failed))]
    #[error(
        "Failed to load template file {}.",
        .path.style(Style::Path),
    )]
    LoadTemplateFileFailed {
        path: PathBuf,
        #[source]
        error: Box<tera::Error>,
    },

    #[diagnostic(code(codegen::template_file::render_failed))]
    #[error(
        "Failed to render template file {}.",
        .path.style(Style::Path),
    )]
    RenderTemplateFileFailed {
        path: PathBuf,
        #[source]
        error: Box<tera::Error>,
    },

    #[diagnostic(code(codegen::template_file::interpolate_path))]
    #[error(
        "Failed to interpolate variables into template file path {}.",
        .path.style(Style::File),
    )]
    InterpolateTemplateFileFailed {
        path: String,
        #[source]
        error: Box<tera::Error>,
    },
}
