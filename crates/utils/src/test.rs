use std::env;
use std::path::PathBuf;

pub fn create_string_vec(list: Vec<&'static str>) -> Vec<String> {
    list.iter().map(|&s| s.to_owned()).collect()
}

pub fn get_fixtures_dir(dir: &str) -> PathBuf {
    get_fixtures_root().join(dir)
}

pub fn get_fixtures_root() -> PathBuf {
    let mut path = env::current_dir().unwrap();
    path.push("../../tests/fixtures");

    path.canonicalize().unwrap()
}
