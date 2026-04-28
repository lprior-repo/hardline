use serde::{Deserialize, Serialize};

use crate::error::QueueError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct QueuePosition(usize);

impl QueuePosition {
    pub fn new(position: usize) -> Self {
        Self(position)
    }

    pub fn front() -> Self {
        Self(0)
    }

    pub fn value(&self) -> usize {
        self.0
    }

    pub fn increment(&self) -> Self {
        Self(self.0 + 1)
    }

    pub fn decrement(&self) -> Result<Self, QueueError> {
        if self.0 == 0 {
            return Err(QueueError::InvalidQueuePosition(
                "cannot decrement below 0".into(),
            ));
        }
        Ok(Self(self.0 - 1))
    }
}

impl Default for QueuePosition {
    fn default() -> Self {
        Self::front()
    }
}

impl From<usize> for QueuePosition {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_position_default_is_front() {
        assert_eq!(QueuePosition::default().value(), 0);
    }

    #[test]
    fn queue_position_increment_increases_value() {
        let pos = QueuePosition::front().increment();
        assert_eq!(pos.value(), 1);
    }

    #[test]
    fn queue_position_decrement_from_zero_fails() {
        let result = QueuePosition::front().decrement();
        assert!(result.is_err());
    }

    #[test]
    fn queue_position_decrement_from_one_succeeds() {
        let result = QueuePosition::front().increment().decrement();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), 0);
    }

    // --- Ordering and comparison ---

    #[test]
    fn queue_position_partial_ord() {
        let a = QueuePosition::new(1);
        let b = QueuePosition::new(5);
        assert!(a < b);
        assert!(b > a);
        assert!(a <= b);
        assert!(b >= a);
    }

    #[test]
    fn queue_position_total_ord() {
        let positions = [
            QueuePosition::new(10),
            QueuePosition::new(1),
            QueuePosition::new(5),
        ];
        let mut sorted = positions;
        sorted.sort();
        assert_eq!(sorted[0].value(), 1);
        assert_eq!(sorted[1].value(), 5);
        assert_eq!(sorted[2].value(), 10);
    }

    #[test]
    fn queue_position_equality() {
        let a = QueuePosition::new(42);
        let b = QueuePosition::new(42);
        assert_eq!(a, b);
    }

    #[test]
    fn queue_position_inequality() {
        let a = QueuePosition::new(1);
        let b = QueuePosition::new(2);
        assert_ne!(a, b);
    }

    // --- From trait ---

    #[test]
    fn queue_position_from_usize() {
        let pos = QueuePosition::from(7_usize);
        assert_eq!(pos.value(), 7);
    }

    #[test]
    fn queue_position_from_zero() {
        let pos = QueuePosition::from(0_usize);
        assert_eq!(pos.value(), 0);
    }

    // --- new constructor ---

    #[test]
    fn queue_position_new_with_value() {
        let pos = QueuePosition::new(999);
        assert_eq!(pos.value(), 999);
    }

    // --- front ---

    #[test]
    fn queue_position_front_value() {
        assert_eq!(QueuePosition::front().value(), 0);
    }

    // --- Multiple increments ---

    #[test]
    fn queue_position_multiple_increments() {
        let pos = QueuePosition::front().increment().increment().increment();
        assert_eq!(pos.value(), 3);
    }

    // --- Increment and decrement roundtrip ---

    #[test]
    fn queue_position_increment_decrement_roundtrip() {
        let start = QueuePosition::new(50);
        let after = start.increment().decrement();
        assert!(after.is_ok());
        assert_eq!(after.unwrap().value(), 50);
    }

    // --- Decrement to exact boundary ---

    #[test]
    fn queue_position_decrement_large_value() {
        let pos = QueuePosition::new(usize::MAX);
        let decremented = pos.decrement();
        assert!(decremented.is_ok());
        assert_eq!(decremented.unwrap().value(), usize::MAX - 1);
    }

    // --- Clone ---

    #[test]
    fn queue_position_clone() {
        let a = QueuePosition::new(10);
        let b = a.clone();
        assert_eq!(a, b);
    }

    // --- Copy semantics ---

    #[test]
    fn queue_position_copy() {
        let a = QueuePosition::new(42);
        let _b = a;
        assert_eq!(a.value(), 42);
    }

    // --- Serde roundtrip ---

    #[test]
    fn queue_position_serde_roundtrip() {
        let pos = QueuePosition::new(123);
        let json = serde_json::to_string(&pos).unwrap();
        let back: QueuePosition = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value(), 123);
    }

    #[test]
    fn queue_position_serde_roundtrip_zero() {
        let pos = QueuePosition::front();
        let json = serde_json::to_string(&pos).unwrap();
        let back: QueuePosition = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value(), 0);
    }

    // --- Debug ---

    #[test]
    fn queue_position_debug() {
        let pos = QueuePosition::new(5);
        let debug = format!("{pos:?}");
        assert!(debug.contains("QueuePosition"));
    }

    // --- Decrement error message ---

    #[test]
    fn queue_position_decrement_error_contains_info() {
        let result = QueuePosition::front().decrement();
        assert!(result.is_err());
        let msg = format!("{:?}", result.unwrap_err());
        assert!(msg.contains("InvalidQueuePosition"));
    }

    // --- PartialEq / Eq consistency ---

    #[test]
    fn queue_position_eq_reflexive() {
        let pos = QueuePosition::new(7);
        assert_eq!(pos, pos);
    }

    #[test]
    fn queue_position_eq_symmetric() {
        let a = QueuePosition::new(7);
        let b = QueuePosition::new(7);
        assert_eq!(a, b);
        assert_eq!(b, a);
    }

    #[test]
    fn queue_position_eq_transitive() {
        let a = QueuePosition::new(7);
        let b = QueuePosition::new(7);
        let c = QueuePosition::new(7);
        assert_eq!(a, b);
        assert_eq!(b, c);
        assert_eq!(a, c);
    }
}
