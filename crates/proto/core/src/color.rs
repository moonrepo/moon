// Colors based on 4th column, except for gray:
// https://upload.wikimedia.org/wikipedia/commons/1/15/Xterm_256color_chart.svg

pub use console::style;
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

pub fn invalid<T: AsRef<str>>(value: T) -> String {
    paint(Color::Yellow as u8, value)
}

pub fn file<T: AsRef<str>>(path: T) -> String {
    paint(Color::Teal as u8, path)
}

pub fn path<T: AsRef<Path>>(path: T) -> String {
    paint(
        Color::Cyan as u8,
        path.as_ref().to_str().unwrap_or("<unknown>"),
    )
}

pub fn url<T: AsRef<str>>(url: T) -> String {
    paint(Color::Blue as u8, url)
}

pub fn shell<T: AsRef<str>>(cmd: T) -> String {
    paint(Color::Pink as u8, cmd)
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
