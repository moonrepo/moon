use std::sync::OnceLock;

pub fn enable_pkl_configs() {
    std::env::set_var("MOON_EXPERIMENT_PKL_CONFIG", "true");
}

pub fn supports_pkl_configs() -> bool {
    static PKL_CACHE: OnceLock<bool> = OnceLock::new();

    *PKL_CACHE.get_or_init(|| {
        std::env::var("MOON_EXPERIMENT_PKL_CONFIG")
            .is_ok_and(|value| value == "1" || value == "true")
    })
}
