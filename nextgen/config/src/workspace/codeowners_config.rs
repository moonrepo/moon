use schematic::{derive_enum, Config, ConfigEnum};

derive_enum!(
    #[derive(ConfigEnum, Copy, Default)]
    pub enum CodeownersOrderBy {
        #[default]
        FileSource,
        ProjectName,
    }
);

#[derive(Config)]
pub struct CodeownersConfig {
    pub order_by: CodeownersOrderBy,

    pub sync_on_run: bool,
}
