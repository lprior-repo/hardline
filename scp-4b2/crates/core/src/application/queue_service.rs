//! Queue Application Service

use crate::error::Result;

pub trait QueueService: Send + Sync {
    fn list_pending(&self) -> Result<Vec<String>>;
}

pub struct QueueServiceImpl;

impl QueueServiceImpl {
    pub fn new() -> Self {
        Self
    }
}

impl QueueService for QueueServiceImpl {
    fn list_pending(&self) -> Result<Vec<String>> {
        Ok(vec![])
    }
}

impl Default for QueueServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_queue_service() -> impl QueueService {
    QueueServiceImpl::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_service() {
        let service = create_queue_service();
        let pending = service.list_pending().unwrap();
        assert!(pending.is_empty());
    }
}
