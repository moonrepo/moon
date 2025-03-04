// HashMap is required for Tera
#![allow(clippy::disallowed_types)]

use convert_case::{Case, Casing};
use moon_common::path::{PathExt, RelativePathBuf};
use starbase_utils::json::{JsonValue as Value, serde_json::to_value};
use std::collections::HashMap;
use std::path::PathBuf;
use tera::{Error, Result, try_get_value};

// STRINGS

fn to_case(case_fn: &str, case_type: Case, value: &Value) -> Result<Value> {
    let s = try_get_value!(case_fn, "value", String, value);

    Ok(to_value(s.to_case(case_type)).unwrap())
}

pub fn camel_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    to_case("camel_case", Case::Camel, value)
}

pub fn kebab_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    to_case("kebab_case", Case::Kebab, value)
}

pub fn upper_kebab_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    to_case("upper_kebab_case", Case::UpperKebab, value)
}

pub fn pascal_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    to_case("pascal_case", Case::Pascal, value)
}

pub fn snake_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    to_case("snake_case", Case::Snake, value)
}

pub fn upper_snake_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    to_case("upper_snake_case", Case::UpperSnake, value)
}

pub fn lower_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("lower_case", "value", String, value);

    Ok(to_value(s.to_lowercase()).unwrap())
}

pub fn upper_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("upper_case", "value", String, value);

    Ok(to_value(s.to_uppercase()).unwrap())
}

// PATHS

pub fn path_join(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let base = try_get_value!("path_join", "value", PathBuf, value);

    let part = match args.get("part") {
        Some(val) => try_get_value!("path_join", "part", RelativePathBuf, val),
        None => return Err(Error::msg("Expected a `part` for `path_join`.")),
    };

    Ok(to_value(part.normalize().to_logical_path(base)).unwrap())
}

pub fn path_relative(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let base = try_get_value!("path_relative", "value", PathBuf, value);

    if args.get("to").is_none() && args.get("from").is_none() {
        return Err(Error::msg("Expected a `to` or `from` for `path_relative`."));
    }

    let rel_to = match args.get("to") {
        Some(val) => try_get_value!("path_relative", "to", PathBuf, val)
            .relative_to(&base)
            .ok(),
        None => None,
    };

    let rel_from = match args.get("from") {
        Some(val) => base
            .relative_to(try_get_value!("path_relative", "from", PathBuf, val))
            .ok(),
        None => None,
    };

    let mut rel = rel_to
        .unwrap_or_else(|| rel_from.unwrap_or(RelativePathBuf::from(".")))
        .normalize();

    if rel.as_str().is_empty() {
        rel = RelativePathBuf::from(".");
    }

    Ok(to_value(rel).unwrap())
}
