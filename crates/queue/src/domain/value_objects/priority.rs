use serde::{Deserialize, Serialize};

use crate::error::QueueError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Priority(u8);

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Invert: higher numeric value = higher priority = sorts first (lower ordinal)
        // This matches the domain convention: "lower number = higher priority"
        // so critical(255) < high(230) < normal(200) < low(100) in sort order
        other.0.cmp(&self.0)
    }
}

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
        Self(230)
    }

    pub fn critical() -> Self {
        Self(u8::MAX)
    }

    pub fn value(&self) -> u8 {
        self.0
    }

    pub fn parse(value: u8) -> Result<Self, QueueError> {
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
    fn priority_high_is_230() {
        assert_eq!(Priority::high().value(), 230);
    }

    #[test]
    fn priority_critical_is_255() {
        assert_eq!(Priority::critical().value(), 255);
    }

    #[test]
    fn priority_ord_higher_number_sorts_first() {
        // Domain convention: higher numeric = higher priority = sorts first
        assert!(Priority::critical() < Priority::high());
        assert!(Priority::high() < Priority::normal());
        assert!(Priority::normal() < Priority::low());
    }

    #[test]
    fn priority_new_arbitrary_value() {
        let p = Priority::new(42);
        assert_eq!(p.value(), 42);
    }

    #[test]
    fn priority_zero() {
        let p = Priority::new(0);
        assert_eq!(p.value(), 0);
    }

    #[test]
    fn priority_parse_ok() {
        let p = Priority::parse(150);
        assert!(p.is_ok());
        assert_eq!(p.unwrap().value(), 150);
    }

    #[test]
    fn priority_from_u8() {
        let p: Priority = 77.into();
        assert_eq!(p.value(), 77);
    }

    #[test]
    fn priority_debug() {
        let p = Priority::critical();
        let debug = format!("{p:?}");
        assert!(debug.contains("Priority"));
    }

    #[test]
    fn priority_serde_roundtrip() {
        let p = Priority::normal();
        let json = serde_json::to_string(&p).unwrap();
        let back: Priority = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value(), 200);
    }

    #[test]
    fn priority_clone_and_copy() {
        let a = Priority::normal();
        let b = a;
        assert_eq!(a.value(), b.value());
    }

    #[test]
    fn priority_equality() {
        let a = Priority::new(100);
        let b = Priority::new(100);
        assert_eq!(a, b);
    }

    #[test]
    fn priority_copy_semantics() {
        let a = Priority::normal();
        let _b = a;
        // a is still usable because Priority: Copy
        assert_eq!(a.value(), 200);
    }

    #[test]
    fn priority_ordering_total() {
        // Higher numeric value = higher priority = sorts first
        assert!(Priority::new(0) > Priority::low());
        assert!(Priority::low() <= Priority::low());
        assert!(Priority::critical() < Priority::normal());
        assert!(Priority::critical() <= Priority::critical());
    }

    #[test]
    fn priority_max_value() {
        assert_eq!(Priority::critical().value(), u8::MAX);
    }
}
