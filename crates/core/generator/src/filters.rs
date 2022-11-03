// HashMap is required for Tera
#![allow(clippy::disallowed_types)]

use convert_case::{Case, Casing};
use serde_json::value::{to_value, Value};
use std::collections::HashMap;
use tera::{try_get_value, Result};

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
