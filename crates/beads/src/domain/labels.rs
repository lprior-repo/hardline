use serde::{Deserialize, Serialize};

/// Labels - collection of labels for a bead
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Labels(pub Vec<String>);

impl Labels {
    #[must_use]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with(mut self, label: impl Into<String>) -> Self {
        self.0.push(label.into());
        self
    }

    #[must_use]
    pub fn contains(&self, label: &str) -> bool {
        self.0.iter().any(|l| l == label)
    }

    #[must_use]
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }
}

impl Default for Labels {
    fn default() -> Self {
        Self::new()
    }
}
