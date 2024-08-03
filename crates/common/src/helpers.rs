use std::sync::OnceLock;

pub fn supports_pkl_configs() -> bool {
    static PKL_CACHE: OnceLock<bool> = OnceLock::new();

    *PKL_CACHE.get_or_init(|| {
        std::env::var("MOON_EXPERIMENT_PKL_CONFIG")
            .is_ok_and(|value| value == "1" || value == "true")
    })
}

pub fn get_config_file_label(file: &str, top_level: bool) -> String {
    let mut label = String::new();

    if top_level {
        label.push_str(crate::consts::CONFIG_DIRNAME);
        label.push('/');
    }

    label.push_str(file);

    if supports_pkl_configs() {
        label.push_str(".{plk,yml}");
    } else {
        label.push_str(".yml");
    }

    label
}
