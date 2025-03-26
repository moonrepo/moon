use moon_config::*;
use rustc_hash::FxHashMap;
use schematic::schema::json_schema::{JsonSchemaOptions, JsonSchemaRenderer};
use schematic::schema::{BooleanType, SchemaField, SchemaGenerator, UnionType};
use schematic::{Schema, SchemaType};
use std::path::Path;

fn create_jsonschema_renderer() -> JsonSchemaRenderer {
    JsonSchemaRenderer::new(JsonSchemaOptions {
        markdown_description: true,
        mark_struct_fields_required: false,
        set_field_name_as_title: true,
        ..JsonSchemaOptions::default()
    })
}

fn generate_project(out_dir: &Path, toolchains: &FxHashMap<String, Schema>) -> miette::Result<()> {
    let mut generator = SchemaGenerator::default();

    // Must come before `ProjectConfig`
    for schema in toolchains.values() {
        if schema.name.is_some() {
            generator.add_schema(schema);
        }
    }

    generator.add::<ProjectConfig>();

    // Inject the currently enabled toolchains into `ProjectToolchainConfig`
    if !toolchains.is_empty() {
        if let Some(config) = generator.schemas.get_mut("ProjectToolchainConfig") {
            if let SchemaType::Struct(inner) = &mut config.ty {
                for (id, schema) in toolchains {
                    inner.fields.insert(
                        id.to_string(),
                        Box::new(SchemaField {
                            comment: Some(format!(
                                "Overrides top-level `{id}` toolchain settings."
                            )),
                            schema: Schema::union(UnionType::new_any([
                                schema.to_owned(),
                                Schema::boolean(BooleanType::new(true)),
                                Schema::null(),
                            ])),
                            nullable: true,
                            ..Default::default()
                        }),
                    );
                }
            }
        }
    }

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

fn generate_toolchain(
    out_dir: &Path,
    toolchains: &FxHashMap<String, Schema>,
) -> miette::Result<()> {
    let mut generator = SchemaGenerator::default();

    // Must come before `ToolchainConfig`
    for schema in toolchains.values() {
        if schema.name.is_some() {
            generator.add_schema(schema);
        }
    }

    generator.add::<ToolchainConfig>();

    // Inject the currently enabled toolchains into `ToolchainConfig`
    if !toolchains.is_empty() {
        if let Some(config) = generator.schemas.get_mut("ToolchainConfig") {
            if let SchemaType::Struct(inner) = &mut config.ty {
                for (id, schema) in toolchains {
                    inner.fields.insert(
                        id.to_string(),
                        Box::new(SchemaField {
                            comment: Some(schema.description.clone().unwrap_or_else(|| {
                                format!("Configures and enables the `{id}` toolchain.")
                            })),
                            schema: {
                                // Make it optional like built-in toolchains
                                let mut schema = schema.to_owned();
                                schema.nullify();
                                schema
                            },
                            nullable: true,
                            ..Default::default()
                        }),
                    );
                }
            }
        }
    }

    generator.generate(out_dir.join("toolchain.json"), create_jsonschema_renderer())
}

fn generate_workspace(out_dir: &Path) -> miette::Result<()> {
    let mut generator = SchemaGenerator::default();
    generator.add::<WorkspaceConfig>();

    let pipeline_config = generator.schemas.get("PipelineConfig").cloned().unwrap();

    if let Some(config) = generator.schemas.get_mut("WorkspaceConfig") {
        if let SchemaType::Struct(inner) = &mut config.ty {
            inner.fields.insert(
                "runner".into(),
                Box::new(SchemaField {
                    deprecated: Some("Use `pipeline` instead.".into()),
                    schema: pipeline_config,
                    nullable: true,
                    ..Default::default()
                }),
            );
        }
    }

    generator.generate(out_dir.join("workspace.json"), create_jsonschema_renderer())
}

pub fn generate_json_schemas(
    out_dir: impl AsRef<Path>,
    toolchain_schemas: FxHashMap<String, Schema>,
) -> miette::Result<bool> {
    let out_dir = out_dir.as_ref();

    generate_project(out_dir, &toolchain_schemas)?;
    generate_tasks(out_dir)?;
    generate_template(out_dir)?;
    generate_toolchain(out_dir, &toolchain_schemas)?;
    generate_workspace(out_dir)?;

    Ok(true)
}
