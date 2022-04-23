pub use dirs::home_dir as get_home_dir;

pub fn replace_home_dir(value: &str) -> String {
    if let Some(home_dir) = get_home_dir() {
        return value.replace(home_dir.to_str().unwrap(), "~");
    }

    value.to_owned()
}
