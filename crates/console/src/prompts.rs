use crate::console::Console;
use inquire::error::InquireResult;
use inquire::ui::{
    Attributes, Color as UiColor, ErrorMessageRenderConfig, RenderConfig, StyleSheet, Styled,
};
use moon_common::color::{no_color, Color};
use std::fmt::Display;

pub use inquire::*;

// Add implementations specific to prompts so that they work together
// without much issue. The biggest problem is that problems write
// directly to stderr, while our logs are buffered. To work around this
// we flush stderr before prompting.
//
// It also allows us to apply styles to all prompts from here instead
// of each callsite, which is super nice!
impl Console {
    fn handle_prompt<T>(&self, result: InquireResult<T>) -> miette::Result<T> {
        result.map_err(|error| miette::miette!(code = "console::prompt", "{}", error.to_string()))
    }

    pub fn confirm(&self, prompt: Confirm) -> miette::Result<bool> {
        self.err.flush()?;
        self.handle_prompt(prompt.with_render_config(*self.theme()).prompt())
    }

    pub fn prompt_custom<T: Clone>(&self, prompt: CustomType<T>) -> miette::Result<T> {
        self.err.flush()?;
        self.handle_prompt(prompt.with_render_config(*self.theme()).prompt())
    }

    pub fn prompt_multiselect<T: Display>(&self, prompt: MultiSelect<T>) -> miette::Result<Vec<T>> {
        self.err.flush()?;
        self.handle_prompt(prompt.with_render_config(*self.theme()).prompt())
    }

    pub fn prompt_select<T: Display>(&self, prompt: Select<T>) -> miette::Result<T> {
        self.err.flush()?;
        self.handle_prompt(prompt.with_render_config(*self.theme()).prompt())
    }

    pub fn prompt_select_skippable<T: Display>(
        &self,
        prompt: Select<T>,
    ) -> miette::Result<Option<T>> {
        self.err.flush()?;
        self.handle_prompt(prompt.with_render_config(*self.theme()).prompt_skippable())
    }

    pub fn prompt_text(&self, prompt: Text) -> miette::Result<String> {
        self.err.flush()?;
        self.handle_prompt(prompt.with_render_config(*self.theme()).prompt())
    }
}

fn rgb(color: Color) -> UiColor {
    UiColor::AnsiValue(color as u8)
}

pub fn create_theme() -> RenderConfig<'static> {
    if no_color() {
        return RenderConfig::empty()
            .with_answer(StyleSheet::new().with_attr(Attributes::BOLD))
            .with_prompt_prefix(Styled::new("›"))
            .with_answered_prompt_prefix(Styled::new("✔"))
            .with_scroll_up_prefix(Styled::new("▴"))
            .with_scroll_down_prefix(Styled::new("▾"))
            .with_highlighted_option_prefix(Styled::new("›"))
            .with_canceled_prompt_indicator(Styled::new("(skipped)"))
            .with_selected_checkbox(Styled::new("◉"))
            .with_unselected_checkbox(Styled::new("◯"))
            .with_error_message(ErrorMessageRenderConfig::empty().with_prefix(Styled::new("✘")));
    }

    RenderConfig::empty()
        // Inputs
        .with_default_value(StyleSheet::new().with_fg(rgb(Color::Pink)))
        .with_answer(
            StyleSheet::new()
                .with_fg(rgb(Color::Purple))
                .with_attr(Attributes::BOLD),
        )
        // Prefixes
        .with_prompt_prefix(Styled::new("›").with_fg(rgb(Color::Blue)))
        .with_answered_prompt_prefix(Styled::new("✔").with_fg(rgb(Color::Green)))
        .with_scroll_up_prefix(Styled::new("▴").with_fg(rgb(Color::GrayLight)))
        .with_scroll_down_prefix(Styled::new("▾").with_fg(rgb(Color::GrayLight)))
        .with_highlighted_option_prefix(Styled::new("›").with_fg(rgb(Color::Teal)))
        // States
        .with_help_message(StyleSheet::new().with_fg(rgb(Color::Purple)))
        .with_error_message(
            ErrorMessageRenderConfig::empty()
                .with_prefix(Styled::new("✘").with_fg(rgb(Color::Red)))
                .with_message(StyleSheet::new().with_fg(rgb(Color::Red))),
        )
        .with_canceled_prompt_indicator(Styled::new("(skipped)").with_fg(rgb(Color::Gray)))
        // Selects
        .with_selected_option(Some(StyleSheet::new().with_fg(rgb(Color::Teal))))
        .with_selected_checkbox(Styled::new("◉").with_fg(rgb(Color::Teal)))
        .with_unselected_checkbox(Styled::new("◯").with_fg(rgb(Color::GrayLight)))
}
