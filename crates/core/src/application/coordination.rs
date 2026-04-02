//! Coordination Application Service

use crate::error::Result;

pub trait CoordinationService: Send + Sync {
    fn list_locks(&self) -> Result<Vec<String>>;
}

pub struct CoordinationServiceImpl;

impl CoordinationServiceImpl {
    pub fn new() -> Self {
        Self
    }
}

impl CoordinationService for CoordinationServiceImpl {
    fn list_locks(&self) -> Result<Vec<String>> {
        Ok(vec![])
    }
}

impl Default for CoordinationServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_coordination_service() -> impl CoordinationService {
    CoordinationServiceImpl::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordination_service() {
        let service = create_coordination_service();
        let locks = service.list_locks().unwrap();
        assert!(locks.is_empty());
    }

    #[test]
    fn given_new_when_called_then_creates_service() {
        let service = CoordinationServiceImpl::new();
        let locks = service.list_locks().unwrap();
        assert!(locks.is_empty());
    }

    #[test]
    fn given_default_when_called_then_same_as_new() {
        let from_new = CoordinationServiceImpl::new();
        let from_default = CoordinationServiceImpl::default();
        let locks_new = from_new.list_locks().unwrap();
        let locks_default = from_default.list_locks().unwrap();
        assert_eq!(locks_new, locks_default);
    }

    #[test]
    fn given_service_when_used_as_trait_object_then_works() {
        let service: Box<dyn CoordinationService> = Box::new(CoordinationServiceImpl::new());
        let locks = service.list_locks().unwrap();
        assert!(locks.is_empty());
    }

    #[test]
    fn given_service_when_list_locks_then_returns_vec() {
        let service = create_coordination_service();
        let locks = service.list_locks().unwrap();
        assert_eq!(locks, Vec::<String>::new());
    }

    #[test]
    fn given_service_when_list_locks_multiple_times_then_consistent() {
        let service = create_coordination_service();
        let first = service.list_locks().unwrap();
        let second = service.list_locks().unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn given_service_when_list_locks_then_result_is_ok() {
        let service = create_coordination_service();
        assert!(service.list_locks().is_ok());
    }

    #[test]
    fn given_service_when_send_sync_then_compiles() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CoordinationServiceImpl>();
    }
}
