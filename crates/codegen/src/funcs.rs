// HashMap is required for Tera
#![allow(clippy::disallowed_types)]

use starbase_utils::json::{JsonMap, JsonValue as Value};
use std::collections::HashMap;
use tera::Result;

pub fn variables(args: &HashMap<String, Value>) -> Result<Value> {
    let mut map = JsonMap::with_capacity(args.len());
    map.extend(args.clone());

    Ok(Value::Object(map))
}
