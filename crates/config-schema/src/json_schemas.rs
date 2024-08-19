use moon_config::*;
use schematic::schema::json_schema::{JsonSchemaOptions, JsonSchemaRenderer};
use schematic::schema::SchemaGenerator;
use std::path::Path;

fn create_jsonschema_renderer() -> JsonSchemaRenderer {
    JsonSchemaRenderer::new(JsonSchemaOptions {
        markdown_description: true,
        mark_struct_fields_required: false,
        set_field_name_as_title: true,
        ..JsonSchemaOptions::default()
    })
}

fn generate_project(out_dir: &Path) -> miette::Result<()> {
    let mut generator = SchemaGenerator::default();
    generator.add::<ProjectConfig>();
    generator.generate(out_dir.join("project.json"), create_jsonschema_renderer())
}

fn generate_tasks(out_dir: &Path) -> miette::Result<()> {
    let mut generator = SchemaGenerator::default();
    generator.add::<InheritedTasksConfig>();
    generator.generate(out_dir.join("tasks.json"), create_jsonschema_renderer())
}

fn generate_template(out_dir: &Path) -> miette::Result<()> {
    let mut generator = SchemaGenerator::default();
    generator.add::<TemplateConfig>();
    generator.generate(out_dir.join("template.json"), create_jsonschema_renderer())?;

    let mut generator = SchemaGenerator::default();
    generator.add::<TemplateFrontmatterConfig>();
    generator.generate(
        out_dir.join("template-frontmatter.json"),
        create_jsonschema_renderer(),
    )
}

fn generate_toolchain(out_dir: &Path) -> miette::Result<()> {
    let mut generator = SchemaGenerator::default();
    generator.add::<ToolchainConfig>();
    generator.generate(out_dir.join("toolchain.json"), create_jsonschema_renderer())
}

fn generate_workspace(out_dir: &Path) -> miette::Result<()> {
    let mut generator = SchemaGenerator::default();
    generator.add::<WorkspaceConfig>();
    generator.generate(out_dir.join("workspace.json"), create_jsonschema_renderer())
}

pub fn generate_json_schemas(out_dir: impl AsRef<Path>) -> miette::Result<()> {
    let out_dir = out_dir.as_ref();

    generate_project(out_dir)?;
    generate_tasks(out_dir)?;
    generate_template(out_dir)?;
    generate_toolchain(out_dir)?;
    generate_workspace(out_dir)?;

    Ok(())
}
