// Colors based on 4th column, except for gray:
// https://upload.wikimedia.org/wikipedia/commons/1/15/Xterm_256color_chart.svg

use ansi_term::Color::Fixed;
use std::path::Path;

pub fn file_path(path: &Path) -> String {
    // Teal
    Fixed(38).paint(path.to_string_lossy()).to_string()
}

pub fn url(url: &str) -> String {
    // Blue
    Fixed(39).paint(url).to_string()
}

pub fn shell(url: &str) -> String {
    // Pink
    Fixed(183).paint(url).to_string()
}

pub fn symbol(url: &str) -> String {
    // Purple
    Fixed(111).paint(url).to_string()
}

pub fn muted(value: &str) -> String {
    // Gray
    Fixed(238).paint(value).to_string()
}

// Based on https://github.com/debug-js/debug/blob/master/src/common.js#L41
pub fn target(value: &str) -> String {
    let mut hash: u32 = 0;

    for c in value.chars() {
        hash = ((hash << 5) - hash) + c.to_digit(10).unwrap();
    }

    // Lot of casting going on here...
    let index = i32::abs(hash as i32) as usize % COLOR_LIST.len();
    let rgb = COLOR_LIST[index];

    Fixed(rgb).paint(value).to_string()
}

pub const COLOR_LIST: [u8; 76] = [
    20, 21, 26, 27, 32, 33, 38, 39, 40, 41, 42, 43, 44, 45, 56, 57, 62, 63, 68, 69, 74, 75, 76, 77,
    78, 79, 80, 81, 92, 93, 98, 99, 112, 113, 128, 129, 134, 135, 148, 149, 160, 161, 162, 163,
    164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 178, 179, 184, 185, 196, 197, 198, 199, 200,
    201, 202, 203, 204, 205, 206, 207, 208, 209, 214, 215, 220, 221,
];

pub const COLOR_LIST_UNSUPPORTED: [u8; 6] = [6, 2, 3, 4, 5, 1];
