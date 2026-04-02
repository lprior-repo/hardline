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

    #[test]
    fn given_new_when_called_then_creates_service() {
        let service = QueueServiceImpl::new();
        let pending = service.list_pending().unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn given_default_when_called_then_same_as_new() {
        let from_new = QueueServiceImpl::new();
        let from_default = QueueServiceImpl::default();
        let pending_new = from_new.list_pending().unwrap();
        let pending_default = from_default.list_pending().unwrap();
        assert_eq!(pending_new, pending_default);
    }

    #[test]
    fn given_service_when_used_as_trait_object_then_works() {
        let service: Box<dyn QueueService> = Box::new(QueueServiceImpl::new());
        let pending = service.list_pending().unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn given_service_when_list_pending_then_returns_vec() {
        let service = create_queue_service();
        let pending = service.list_pending().unwrap();
        assert_eq!(pending, Vec::<String>::new());
    }

    #[test]
    fn given_service_when_list_pending_multiple_times_then_consistent() {
        let service = create_queue_service();
        let first = service.list_pending().unwrap();
        let second = service.list_pending().unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn given_service_when_list_pending_then_result_is_ok() {
        let service = create_queue_service();
        assert!(service.list_pending().is_ok());
    }

    #[test]
    fn given_service_when_send_sync_then_compiles() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<QueueServiceImpl>();
    }
}
