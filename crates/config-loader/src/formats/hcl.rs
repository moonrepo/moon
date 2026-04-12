use hcl::{
    Body, Structure, Value,
    de::Deserializer,
    eval::{Context, Evaluate, FuncArgs, FuncDef, ParamType},
    value::Map,
};
use miette::NamedSource;
use schematic::{ConfigError, ParserError, Source, SourceFormat};
use serde::de::DeserializeOwned;
use std::path::Path;

#[derive(Default)]
pub struct HclFormat {}

impl HclFormat {
    pub fn create_context(&self) -> Context<'_> {
        let mut ctx = Context::new();

        ctx.declare_func(
            "concat",
            FuncDef::builder()
                .param(ParamType::array_of(ParamType::Any))
                .param(ParamType::array_of(ParamType::Any))
                .build(concat),
        );

        ctx
    }

    pub fn inherit_variables(&self, body: Body, ctx: &mut Context) -> Body {
        let mut builder = Body::builder();

        for structure in body.into_inner() {
            match structure {
                Structure::Attribute(attr) => {
                    builder = builder.add_attribute(attr);
                }
                Structure::Block(block) => {
                    if block.identifier.as_str() == "locals" {
                        let mut local = Map::default();

                        for attr in block.body().attributes() {
                            local.insert(attr.key().to_owned(), attr.expr().evaluate(ctx).unwrap());
                        }

                        ctx.declare_var("local", Value::Object(local));
                    } else {
                        builder = builder.add_block(block);
                    }
                }
            };
        }

        builder.build()
    }
}

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
        let mut ctx = self.create_context();

        let body = hcl::parse(content).map_err(|error| ParserError {
            content: NamedSource::new(name, content.to_owned()),
            path: error.to_string(),
            span: None,
            message: error.to_string(),
        })?;

        let de = Deserializer::from_body(
            self.inherit_variables(body, &mut ctx)
                .evaluate(&ctx)
                .map_err(|error| ParserError {
                    content: NamedSource::new(name, content.to_owned()),
                    path: error.expr().map(|ex| ex.to_string()).unwrap_or_default(),
                    span: None,
                    message: error.to_string(),
                })?,
        );

        let result: T = serde_path_to_error::deserialize(de).map_err(|error| ParserError {
            content: NamedSource::new(name, content.to_owned()),
            path: error.path().to_string(),
            span: None,
            message: error.inner().to_string(),
        })?;

        Ok(result)
    }
}

fn concat(args: FuncArgs) -> Result<Value, String> {
    let mut args = args.into_values();

    match (args.remove(0), args.remove(0)) {
        (Value::Array(l), Value::Array(r)) => Ok(Value::Array({
            let mut list = vec![];
            list.extend(l);
            list.extend(r);
            list
        })),
        _ => unreachable!(),
    }
}
