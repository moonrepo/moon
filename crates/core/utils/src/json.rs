use ec4rs::property::*;
use moon_error::{map_io_to_fs_error, MoonError};
use std::fs;
use std::path::Path;

pub use json::{from, parse, JsonValue};

pub fn write<P: AsRef<Path>>(path: P, json: JsonValue, pretty: bool) -> Result<(), MoonError> {
    let path = path.as_ref();

    if !pretty {
        fs::write(path, json::stringify(json))
            .map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;

        return Ok(());
    }

    let editor_config = ec4rs::properties_of(path).unwrap_or_default();
    let indent_size = editor_config
        .get::<IndentSize>()
        .unwrap_or(IndentSize::Value(2));
    let insert_final_newline = editor_config
        .get::<FinalNewline>()
        .unwrap_or(FinalNewline::Value(true));

    // json crate doesnt support tabs, so always use space indentation
    let spaces = match indent_size {
        IndentSize::UseTabWidth => 2,
        IndentSize::Value(value) => value,
    };

    let mut data = json::stringify_pretty(json, spaces as u16);

    if matches!(insert_final_newline, FinalNewline::Value(true)) {
        data += "\n";
    }

    fs::write(path, data).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;

    return Ok(());
}
