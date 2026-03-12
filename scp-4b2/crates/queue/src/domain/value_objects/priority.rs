use crate::error::QueueError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Priority(u8);

impl Priority {
    pub fn new(value: u8) -> Self {
        Self(value)
    }

    pub fn low() -> Self {
        Self(100)
    }

    pub fn normal() -> Self {
        Self(200)
    }

    pub fn high() -> Self {
        Self(300)
    }

    pub fn critical() -> Self {
        Self(255)
    }

    pub fn value(&self) -> u8 {
        self.0
    }

    pub fn parse(value: u8) -> Result<Self, QueueError> {
        if value > 255 {
            return Err(QueueError::InvalidPriority(value.into()));
        }
        Ok(Self(value))
    }
}

impl Default for Priority {
    fn default() -> Self {
        Self::normal()
    }
}

impl From<u8> for Priority {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_default_is_normal() {
        assert_eq!(Priority::default().value(), 200);
    }

    #[test]
    fn priority_low_is_100() {
        assert_eq!(Priority::low().value(), 100);
    }

    #[test]
    fn priority_high_is_300() {
        assert_eq!(Priority::high().value(), 300);
    }

    #[test]
    fn priority_critical_is_255() {
        assert_eq!(Priority::critical().value(), 255);
    }

    #[test]
    fn priority_ord_compares_by_value() {
        assert!(Priority::low() < Priority::normal());
        assert!(Priority::normal() < Priority::high());
    }
}
