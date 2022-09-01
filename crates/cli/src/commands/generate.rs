use crate::helpers::load_workspace;
use moon_generator::Generator;
use moon_logger::color;

pub struct GenerateOptions {
    pub template: bool,
}

pub async fn generate(
    name: &str,
    options: GenerateOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;
    let generator = Generator::create(&workspace.root, &workspace.config.generator)?;

    // This is a special case for creating a new template with the generator itself!
    if options.template {
        let template = generator.generate_template(name).await?;

        println!(
            "Created a new template {} at {}",
            color::id(template.name),
            color::path(template.root)
        );

        return Ok(());
    }

    Ok(())
}
