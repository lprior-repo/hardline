#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Priority(u8);

impl Priority {
    pub fn new(value: u8) -> Result<Self, super::job_id::JobCreationError> {
        Ok(Self(value))
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_valid() {
        let p = Priority::new(100);
        assert!(p.is_ok());
        assert_eq!(p.unwrap().value(), 100);
    }

    #[test]
    fn priority_max_value() {
        let p = Priority::new(255);
        assert!(p.is_ok());
    }
}
