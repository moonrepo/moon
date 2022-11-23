// HashMap is required for Tera
#![allow(clippy::disallowed_types)]

use convert_case::{Case, Casing};
use moon_utils::path;
use serde_json::value::{to_value, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use tera::{try_get_value, Error, Result};

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
    to_case("lower_case", Case::Lower, value)
}

pub fn upper_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    to_case("upper_case", Case::Upper, value)
}

// PATHS

pub fn path_join(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let base = try_get_value!("path_join", "value", PathBuf, value);

    let part = match args.get("part") {
        Some(val) => try_get_value!("path_join", "part", String, val),
        None => return Err(Error::msg("Expected a `part` for `path_join`.")),
    };

    let full = path::to_virtual_string(path::normalize(base.join(part)))
        .map_err(|e| Error::msg(e.to_string()))?;

    Ok(to_value(full).unwrap())
}

pub fn path_relative(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let base = try_get_value!("path_relative", "value", PathBuf, value);

    if args.get("to").is_none() && args.get("from").is_none() {
        return Err(Error::msg("Expected a `to` or `from` for `path_relative`."));
    }

    let rel_to = match args.get("to") {
        Some(val) => path::relative_from(try_get_value!("path_relative", "to", String, val), &base),
        None => None,
    };

    let rel_from = match args.get("from") {
        Some(val) => {
            path::relative_from(&base, try_get_value!("path_relative", "from", String, val))
        }
        None => None,
    };

    let rel = rel_to.unwrap_or_else(|| rel_from.unwrap_or(base));
    let full =
        path::to_virtual_string(path::normalize(rel)).map_err(|e| Error::msg(e.to_string()))?;

    Ok(to_value(full).unwrap())
}
