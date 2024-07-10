use miette::IntoDiagnostic;
use moon_common::Id;
use moon_target::Target;
use tera::{Context, Tera};

#[derive(Debug, Default)]
pub struct GenerateDockerfileOptions {
    pub build_task: Option<Target>,
    pub disable_toolchain: bool,
    pub image: String,
    pub project: Id,
    pub prune: bool,
    pub start_task: Option<Target>,
}

pub fn generate_dockerfile(mut options: GenerateDockerfileOptions) -> miette::Result<String> {
    if options.image.is_empty() {
        options.image = "scratch".into();
    }

    if options.image.contains("alpine") {
        options.disable_toolchain = true;
    }

    let mut context = Context::new();
    context.insert("disable_toolchain", &options.disable_toolchain);
    context.insert("image", &options.image);
    context.insert("project", &options.project);
    context.insert("prune", &options.prune);

    if let Some(task) = &options.build_task {
        context.insert("build_task", task);
    }

    if let Some(task) = &options.start_task {
        context.insert("start_task", task);
    }

    let result = Tera::one_off(
        include_str!("../templates/Dockerfile.tera"),
        &context,
        false,
    )
    .into_diagnostic()?;

    Ok(result)
}
