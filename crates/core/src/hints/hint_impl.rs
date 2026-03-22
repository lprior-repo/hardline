//! Hint builder methods
//!
//! Methods for constructing Hint instances

use serde_json;

use super::types::{Hint, HintType};

impl Hint {
    /// Create an info hint
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            hint_type: HintType::Info,
            message: message.into(),
            suggested_command: None,
            rationale: None,
            context: None,
        }
    }

    /// Create a suggestion hint
    pub fn suggestion(message: impl Into<String>) -> Self {
        Self {
            hint_type: HintType::Suggestion,
            message: message.into(),
            suggested_command: None,
            rationale: None,
            context: None,
        }
    }

    /// Create a warning hint
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            hint_type: HintType::Warning,
            message: message.into(),
            suggested_command: None,
            rationale: None,
            context: None,
        }
    }

    /// Create a tip hint
    pub fn tip(message: impl Into<String>) -> Self {
        Self {
            hint_type: HintType::Tip,
            message: message.into(),
            suggested_command: None,
            rationale: None,
            context: None,
        }
    }

    /// Add a suggested command
    #[must_use]
    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.suggested_command = Some(command.into());
        self
    }

    /// Add a rationale
    #[must_use]
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }

    /// Add context
    #[must_use]
    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }
}
