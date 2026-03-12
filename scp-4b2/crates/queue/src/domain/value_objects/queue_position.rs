use crate::error::QueueError;
use serde::{Deserialize, Serialize};

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
}
