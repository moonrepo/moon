// Colors based on 4th column, except for gray:
// https://upload.wikimedia.org/wikipedia/commons/1/15/Xterm_256color_chart.svg

pub use console::style;
use console::{pad_str, Alignment};
use dirs::home_dir as get_home_dir;
use log::Level;
use std::env;
use std::path::Path;

pub enum Color {
    White = 15,
    Black = 16,
    Green = 35,
    Teal = 36,
    Cyan = 38,
    Blue = 39,
    Purple = 111,
    Lime = 112,
    Red = 161,
    Pink = 183,
    Yellow = 185,
    Gray = 239,
    GrayLight = 248,
}

pub fn paint<T: AsRef<str>>(color: u8, value: T) -> String {
    style(value.as_ref()).color256(color).to_string()
}

pub fn muted<T: AsRef<str>>(value: T) -> String {
    paint(Color::Gray as u8, value)
}

pub fn muted_light<T: AsRef<str>>(value: T) -> String {
    paint(Color::GrayLight as u8, value)
}

pub fn success<T: AsRef<str>>(value: T) -> String {
    paint(Color::Green as u8, value)
}

pub fn failure<T: AsRef<str>>(value: T) -> String {
    paint(Color::Red as u8, value)
}

pub fn hash<T: AsRef<str>>(value: T) -> String {
    paint(Color::Green as u8, value)
}

pub fn invalid<T: AsRef<str>>(value: T) -> String {
    paint(Color::Yellow as u8, value)
}

pub fn file<T: AsRef<str>>(path: T) -> String {
    paint(Color::Teal as u8, path)
}

pub fn path<T: AsRef<Path>>(path: T) -> String {
    paint(
        Color::Cyan as u8,
        clean_path(path.as_ref().to_str().unwrap_or("<unknown>")),
    )
}

pub fn url<T: AsRef<str>>(url: T) -> String {
    paint(Color::Blue as u8, url)
}

pub fn shell<T: AsRef<str>>(cmd: T) -> String {
    paint(Color::Pink as u8, clean_path(cmd))
}

pub fn symbol<T: AsRef<str>>(value: T) -> String {
    paint(Color::Lime as u8, value)
}

pub fn id<T: AsRef<str>>(value: T) -> String {
    paint(Color::Purple as u8, value)
}

pub fn target<T: AsRef<str>>(value: T) -> String {
    paint(Color::Blue as u8, value)
}

// Based on https://github.com/debug-js/debug/blob/master/src/common.js#L41
pub fn log_target<T: AsRef<str>>(value: T) -> String {
    let value = value.as_ref();
    let mut hash: u32 = 0;

    for b in value.bytes() {
        hash = (hash << 5).wrapping_sub(hash) + b as u32;
    }

    // Lot of casting going on here...
    if supports_color() >= 2 {
        let index = i32::abs(hash as i32) as usize % COLOR_LIST.len();

        return style(value).color256(COLOR_LIST[index]).to_string();
    }

    let index = i32::abs(hash as i32) as usize % COLOR_LIST_UNSUPPORTED.len();

    style(value)
        .color256(COLOR_LIST_UNSUPPORTED[index])
        .to_string()
}

pub fn log_level(level: Level) -> String {
    let msg = String::from(pad_str(level.as_str(), 5, Alignment::Right, None)).to_lowercase();

    match level {
        // Only color these as we want them to stand out
        Level::Error => paint(Color::Red as u8, &msg),
        Level::Warn => paint(Color::Yellow as u8, &msg),
        _ => muted(&msg),
    }
}

pub fn no_color() -> bool {
    env::var("NO_COLOR").is_ok()
}

// 1 = 8
// 2 = 256
// 3 = 16m
pub fn supports_color() -> u8 {
    if let Ok(var) = env::var("TERM") {
        if var == "dumb" {
            return 1;
        } else if var.contains("truecolor") {
            return 3;
        } else if var.contains("256") {
            return 2;
        }
    }

    if let Ok(var) = env::var("COLORTERM") {
        if var == "truecolor" || var == "24bit" {
            return 3;
        } else {
            return 1;
        }
    }

    2
}

pub const COLOR_LIST: [u8; 76] = [
    20, 21, 26, 27, 32, 33, 38, 39, 40, 41, 42, 43, 44, 45, 56, 57, 62, 63, 68, 69, 74, 75, 76, 77,
    78, 79, 80, 81, 92, 93, 98, 99, 112, 113, 128, 129, 134, 135, 148, 149, 160, 161, 162, 163,
    164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 178, 179, 184, 185, 196, 197, 198, 199, 200,
    201, 202, 203, 204, 205, 206, 207, 208, 209, 214, 215, 220, 221,
];

pub const COLOR_LIST_UNSUPPORTED: [u8; 6] = [6, 2, 3, 4, 5, 1];

fn clean_path<T: AsRef<str>>(path: T) -> String {
    let path = path.as_ref();
    let mut path_str = path.to_owned();

    if let Some(home_dir) = get_home_dir() {
        let home_dir_str = home_dir.to_str().unwrap_or_default();

        if !home_dir_str.is_empty() && path.starts_with(home_dir_str) {
            path_str = path_str.replace(home_dir_str, "~");
        }
    }

    if env::var("MOON_TEST_STANDARDIZE_PATHS").is_ok() {
        path_str = path_str.replace('\\', "/");
    }

    path_str
}
