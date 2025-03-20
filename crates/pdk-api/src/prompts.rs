use serde_json::Value as JsonValue;
use warpgate_api::{api_enum, api_struct};

api_enum!(
    /// The type of prompt to render to receive an answer.
    #[derive(Default)]
    #[serde(tag = "type", rename_all = "kebab-case")]
    pub enum PromptType {
        #[default]
        #[doc(hidden)]
        None,

        /// A confirmation prompt.
        Confirm { default: bool },

        /// A text input field.
        Input { default: String },

        /// A select field with options.
        Select {
            default_index: usize,
            options: Vec<JsonValue>,
        },
    }
);

// api_struct!(
//     pub struct Prompt {
//         pub question: String,
//         pub required: bool,
//         pub ty: PromptType,
//     }
// );

api_struct!(
    /// Represents a prompt (question) for a configuration setting.
    #[serde(default)]
    pub struct SettingPrompt {
        /// A condition to evaluate on whether to render this prompt.
        pub condition: Option<SettingCondition>,

        /// Description of what the setting will do.
        pub description: Option<String>,

        /// Will be rendered in the minimal initialization flow.
        pub minimal: bool,

        /// Nested prompts to render if the answer is truthy.
        pub prompts: Vec<SettingPrompt>,

        /// The question to prompt the user.
        pub question: String,

        /// Whether this prompt is required or optional.
        pub required: bool,

        /// Name of the setting to inject. Supports dot notation.
        pub setting: String,

        /// Skip injecting this setting if the answer is falsy.
        pub skip_if_falsy: bool,

        /// Type of prompt to render.
        pub ty: PromptType,
    }
);

impl SettingPrompt {
    /// Create a new minimal setting.
    pub fn new(setting: impl AsRef<str>, question: impl AsRef<str>, ty: PromptType) -> Self {
        Self {
            minimal: true,
            question: question.as_ref().into(),
            required: true,
            setting: setting.as_ref().into(),
            ty,
            ..Default::default()
        }
    }

    /// Create a new full (non-minimal) setting.
    pub fn new_full(setting: impl AsRef<str>, question: impl AsRef<str>, ty: PromptType) -> Self {
        let mut prompt = Self::new(setting, question, ty);
        prompt.minimal = false;
        prompt
    }
}

api_enum!(
    /// A type of condition to evaluate against a setting value.
    #[derive(Default)]
    #[serde(tag = "op", content = "match", rename_all = "kebab-case")]
    pub enum ConditionType {
        BoolEquals(bool),
        #[default]
        Exists,
        FloatEquals(f64),
        IntEquals(i64),
        NotExists,
        StringContains(String),
        StringEquals(String),
    }
);

api_struct!(
    /// Represents a condition against another setting.
    pub struct SettingCondition {
        pub op: ConditionType,
        pub setting: String,
    }
);
