use hcl::{
    de::Deserializer,
    eval::{Context, Evaluate},
};
use miette::NamedSource;
use schematic::{ConfigError, ParserError, Source, SourceFormat};
use serde::de::DeserializeOwned;
use std::path::Path;

#[derive(Default)]
pub struct HclFormat;

impl<T: DeserializeOwned> SourceFormat<T> for HclFormat {
    fn should_parse(&self, source: &Source) -> bool {
        source.get_file_ext().is_some_and(|ext| ext == "hcl")
    }

    fn parse(
        &self,
        source: &Source,
        content: &str,
        _cache_path: Option<&Path>,
    ) -> Result<T, ConfigError> {
        let name = source.get_file_name();
        let ctx = Context::new();

        let body = hcl::parse(content).map_err(|error| ParserError {
            content: NamedSource::new(name, content.to_owned()),
            path: error.to_string(),
            span: None,
            message: error.to_string(),
        })?;

        let de = Deserializer::from_body(body.evaluate(&ctx).map_err(|error| ParserError {
            content: NamedSource::new(name, content.to_owned()),
            path: error.expr().map(|ex| ex.to_string()).unwrap_or_default(),
            span: None,
            message: error.to_string(),
        })?);

        let result: T = serde_path_to_error::deserialize(de).map_err(|error| ParserError {
            content: NamedSource::new(name, content.to_owned()),
            path: error.path().to_string(),
            span: None,
            message: error.inner().to_string(),
        })?;

        Ok(result)
    }
}
