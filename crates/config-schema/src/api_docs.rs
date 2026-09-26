use moon_config::*;
use schematic::schema::{SchemaGenerator, api_docs::*};
use std::path::Path;

pub fn link_renderer(name: &str) -> String {
    format!("[`{name}`](./{name}.mdx)")
}

pub fn generate_api_docs(out_dir: impl AsRef<Path>) -> miette::Result<bool> {
    let out_dir = out_dir.as_ref();
    let _ = std::fs::remove_dir_all(out_dir);

    let mut generator = SchemaGenerator::default();
    generator.add::<WorkspaceConfig>();
    generator.add::<ToolchainsConfig>();
    generator.add::<ExtensionsConfig>();
    generator.add::<InheritedTasksConfig>();
    generator.add::<ProjectConfig>();
    generator.add::<TemplateConfig>();

    ApiDocsRenderer::new(ApiDocsOptions {
        enum_format: ApiDocsEnumFormat::Table,
        file_extension: "mdx".into(),
        index_page: Some("index.mdx".into()),
        render_link: Box::new(link_renderer),
        ..Default::default()
    })
    .generate_all(&generator, out_dir)?;

    Ok(true)
}
