use moon_config::TemplateConfig;
use moon_constants::CONFIG_TEMPLATE_FILENAME;
use std::path::PathBuf;

fn load_jailed_config() -> Result<TemplateConfig, figment::Error> {
    match TemplateConfig::load(&PathBuf::from(CONFIG_TEMPLATE_FILENAME)) {
        Ok(cfg) => Ok(cfg),
        Err(errors) => Err(errors.first().unwrap().clone()),
    }
}

#[test]
fn empty_file() {
    figment::Jail::expect_with(|jail| {
        // Needs a fake yaml value, otherwise the file reading panics
        jail.create_file(CONFIG_TEMPLATE_FILENAME, "fake: value")?;

        load_jailed_config()?;

        Ok(())
    });
}

mod title {
    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected a string for key \"template.title\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(super::CONFIG_TEMPLATE_FILENAME, "title: 123")?;

            super::load_jailed_config()?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(expected = "Must be a non-empty string for key \"template.title\"")]
    fn min_length() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TEMPLATE_FILENAME,
                "title: ''\ndescription: 'asd'",
            )?;

            super::load_jailed_config()?;

            Ok(())
        });
    }
}

mod description {
    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected a string for key \"template.description\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(super::CONFIG_TEMPLATE_FILENAME, "description: 123")?;

            super::load_jailed_config()?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(expected = "Must be a non-empty string for key \"template.description\"")]
    fn min_length() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TEMPLATE_FILENAME,
                "title: 'asd'\ndescription: ''",
            )?;

            super::load_jailed_config()?;

            Ok(())
        });
    }
}
