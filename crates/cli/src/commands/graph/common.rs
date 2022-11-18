use std::env;

pub fn get_js_url(is_production: bool) -> String {
    match env::var("MOON_JS_URL") {
        Ok(url) => url,
        Err(..) => match is_production {
            false => "http://localhost:5000".to_string(),
            true => "https://unpkg.com/@moonrepo/visualizer-browser@latest".to_string(),
        },
    }
}
