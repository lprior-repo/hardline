//! Conflict resolution configuration

use serde::{Deserialize, Serialize};

use crate::Result;

use super::types::ConflictMode;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConflictResolutionConfig {
    pub mode: ConflictMode,
    pub autonomy: u8,
    pub security_keywords: Vec<String>,
    pub log_resolutions: bool,
}

impl ConflictResolutionConfig {
    pub fn validate(&self) -> Result<()> {
        if self.autonomy > 100 {
            return Err(crate::Error::ValidationError(format!(
                "autonomy must be 0-100, got {}",
                self.autonomy
            )));
        }

        if self.security_keywords.is_empty() {
            return Err(crate::Error::ValidationError(
                "security_keywords must not be empty".to_string(),
            ));
        }

        match self.mode {
            ConflictMode::Auto | ConflictMode::Manual | ConflictMode::Hybrid => Ok(()),
        }
    }

    #[must_use]
    pub fn requires_human_review(&self, file_path: &str) -> bool {
        let file_path_lower = file_path.to_lowercase();
        self.security_keywords
            .iter()
            .any(|keyword| file_path_lower.contains(&keyword.to_lowercase()))
    }

    #[must_use]
    pub fn can_auto_resolve(&self, file_path: Option<&str>) -> bool {
        match self.mode {
            ConflictMode::Auto => true,
            ConflictMode::Manual => false,
            ConflictMode::Hybrid => file_path.map_or(self.autonomy >= 50, |path| {
                !self.requires_human_review(path) && self.autonomy >= 50
            }),
        }
    }
}

impl Default for ConflictResolutionConfig {
    fn default() -> Self {
        Self {
            mode: ConflictMode::Manual,
            autonomy: 0,
            security_keywords: vec![
                "password".to_string(),
                "token".to_string(),
                "secret".to_string(),
                "key".to_string(),
                "credential".to_string(),
            ],
            log_resolutions: true,
        }
    }
}
