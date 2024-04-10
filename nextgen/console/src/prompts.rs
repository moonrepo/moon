use std::fmt::Display;

pub use inquire::*;

use crate::console::Console;
use inquire::error::InquireResult;

// Add implementations specific to prompts so that they work together
// without much issue. The biggest problem is that problems write
// directly to stderr, while our logs are buffered. To work around this
// we flush stderr before prompting.
//
// It also allows us to apply styles to all prompts from here instead
// of each callsite, which is super nice!
impl Console {
    fn handle_prompt<T>(&self, result: InquireResult<T>) -> miette::Result<T> {
        // TODO
        result.map_err(|error| miette::miette!("{}", error.to_string()))
    }

    pub fn confirm(&self, prompt: Confirm) -> miette::Result<bool> {
        self.err.flush()?;
        self.handle_prompt(prompt.prompt())
    }

    pub fn prompt_custom<T: Clone>(&self, prompt: CustomType<T>) -> miette::Result<T> {
        self.err.flush()?;
        self.handle_prompt(prompt.prompt())
    }

    pub fn prompt_multiselect<T: Display>(&self, prompt: MultiSelect<T>) -> miette::Result<Vec<T>> {
        self.err.flush()?;
        self.handle_prompt(prompt.prompt())
    }

    pub fn prompt_select<T: Display>(&self, prompt: Select<T>) -> miette::Result<T> {
        self.err.flush()?;
        self.handle_prompt(prompt.prompt())
    }

    pub fn prompt_text(&self, prompt: Text) -> miette::Result<String> {
        self.err.flush()?;
        self.handle_prompt(prompt.prompt())
    }
}
