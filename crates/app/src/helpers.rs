use console::{style, Style};
use dialoguer::theme::ColorfulTheme;
use indicatif::{ProgressBar, ProgressStyle};
use starbase_styles::color::Color;
use std::time::Duration;

pub fn create_progress_bar<S: AsRef<str>, F: AsRef<str>>(start: S) -> impl FnOnce(F, bool) {
    let pb = ProgressBar::new_spinner();
    pb.set_message(start.as_ref().to_owned());
    pb.enable_steady_tick(Duration::from_millis(50));

    move |finish, passed| {
        let theme = create_theme();

        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{prefix} {msg}")
                .unwrap(),
        );

        if passed {
            pb.set_prefix(theme.success_prefix.to_string());
        } else {
            pb.set_prefix(theme.error_prefix.to_string());
        }

        pb.finish_with_message(finish.as_ref().to_owned());
    }
}

pub fn create_theme() -> ColorfulTheme {
    ColorfulTheme {
        defaults_style: Style::new().for_stderr().color256(Color::Pink as u8),
        prompt_style: Style::new().for_stderr(),
        prompt_prefix: style("?".to_string())
            .for_stderr()
            .color256(Color::Blue as u8),
        prompt_suffix: style("›".to_string())
            .for_stderr()
            .color256(Color::Gray as u8),
        success_prefix: style("✔".to_string())
            .for_stderr()
            .color256(Color::Green as u8),
        success_suffix: style("·".to_string())
            .for_stderr()
            .color256(Color::Gray as u8),
        error_prefix: style("✘".to_string())
            .for_stderr()
            .color256(Color::Red as u8),
        error_style: Style::new().for_stderr().color256(Color::Pink as u8),
        hint_style: Style::new().for_stderr().color256(Color::Purple as u8),
        values_style: Style::new().for_stderr().color256(Color::Purple as u8),
        active_item_style: Style::new().for_stderr().color256(Color::Teal as u8),
        inactive_item_style: Style::new().for_stderr(),
        active_item_prefix: style("❯".to_string())
            .for_stderr()
            .color256(Color::Teal as u8),
        inactive_item_prefix: style(" ".to_string()).for_stderr(),
        checked_item_prefix: style("✔".to_string())
            .for_stderr()
            .color256(Color::Teal as u8),
        unchecked_item_prefix: style("✔".to_string())
            .for_stderr()
            .color256(Color::GrayLight as u8),
        picked_item_prefix: style("❯".to_string())
            .for_stderr()
            .color256(Color::Teal as u8),
        unpicked_item_prefix: style(" ".to_string()).for_stderr(),
    }
}
