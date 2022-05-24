pub fn get_bin_name_suffix(name: &str, windows_ext: &str, flat: bool) -> String {
    if cfg!(windows) {
        format!("{}.{}", name, windows_ext)
    } else if flat {
        name.to_owned()
    } else {
        format!("bin/{}", name)
    }
}
