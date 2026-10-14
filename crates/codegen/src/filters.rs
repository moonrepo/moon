use convert_case::{Case, Casing};
use moon_common::path::{PathExt, RelativePathBuf};
use std::path::PathBuf;
use tera::{Error, Kwargs, State, TeraResult};

// STRINGS

pub fn camel_case(value: &str, _: Kwargs, _: &State) -> String {
    value.to_case(Case::Camel)
}

pub fn kebab_case(value: &str, _: Kwargs, _: &State) -> String {
    value.to_case(Case::Kebab)
}

pub fn upper_kebab_case(value: &str, _: Kwargs, _: &State) -> String {
    value.to_case(Case::UpperKebab)
}

pub fn pascal_case(value: &str, _: Kwargs, _: &State) -> String {
    value.to_case(Case::Pascal)
}

pub fn snake_case(value: &str, _: Kwargs, _: &State) -> String {
    value.to_case(Case::Snake)
}

pub fn upper_snake_case(value: &str, _: Kwargs, _: &State) -> String {
    value.to_case(Case::UpperSnake)
}

pub fn lower_case(value: &str, _: Kwargs, _: &State) -> String {
    value.to_lowercase()
}

pub fn upper_case(value: &str, _: Kwargs, _: &State) -> String {
    value.to_uppercase()
}

// PATHS

pub fn path_join(value: &str, args: Kwargs, _: &State) -> TeraResult<String> {
    let Some(part) = args.get::<&str>("part")? else {
        return Err(Error::message("Expected a `part` for `path_join`."));
    };

    Ok(RelativePathBuf::from(part)
        .normalize()
        .to_logical_path(value)
        .to_string_lossy()
        .into_owned())
}

pub fn path_relative(value: &str, args: Kwargs, _: &State) -> TeraResult<String> {
    let base = PathBuf::from(value);
    let to = args.get::<&str>("to")?;
    let from = args.get::<&str>("from")?;

    if to.is_none() && from.is_none() {
        return Err(Error::message(
            "Expected a `to` or `from` for `path_relative`.",
        ));
    }

    let rel_to = to.and_then(|to| PathBuf::from(to).relative_to(&base).ok());
    let rel_from = from.and_then(|from| base.relative_to(from).ok());

    let mut rel = rel_to
        .unwrap_or_else(|| rel_from.unwrap_or(RelativePathBuf::from(".")))
        .normalize();

    if rel.as_str().is_empty() {
        rel = RelativePathBuf::from(".");
    }

    Ok(rel.into_string())
}
