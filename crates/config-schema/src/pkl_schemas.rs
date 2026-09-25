use moon_config::*;
use schematic::schema::{SchemaGenerator, pkl_schema::*};
use std::path::Path;

pub fn generate_pkl_schemas(out_dir: impl AsRef<Path>) -> miette::Result<bool> {
    let mut generator = SchemaGenerator::default();
    generator.add::<WorkspaceConfig>();
    generator.add::<ToolchainsConfig>();
    generator.add::<ExtensionsConfig>();
    generator.add::<InheritedTasksConfig>();
    generator.add::<ProjectConfig>();
    generator.add::<TemplateConfig>();

    // Configs are merged in layers, so a config must only output the settings
    // it sets, or the defaults of the module would override the layers below
    PklSchemaRenderer::new(PklSchemaOptions {
        mark_struct_fields_required: false,
        ..PklSchemaOptions::default()
    })
    .generate_all(&generator, out_dir)?;

    Ok(true)
}
