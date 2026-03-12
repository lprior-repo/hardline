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
}
