use crate::domain::entities::Stack;
use crate::error::Result;

pub struct StackEngine;

impl StackEngine {
    pub fn load_stack() -> Result<Stack> {
        Err(crate::error::StackError::NotFound(
            "Stack loading not yet implemented".to_string(),
        ))
    }

    pub fn sync_stack() -> Result<Stack> {
        Err(crate::error::StackError::NotFound(
            "Stack sync not yet implemented".to_string(),
        ))
    }

    pub fn restack_branch(_branch: &str) -> Result<()> {
        Err(crate::error::StackError::NotFound(
            "Restack not yet implemented".to_string(),
        ))
    }

    pub fn create_branch(_name: &str, _parent: Option<&str>) -> Result<()> {
        Err(crate::error::StackError::NotFound(
            "Create branch not yet implemented".to_string(),
        ))
    }

    pub fn delete_branch(_name: &str) -> Result<()> {
        Err(crate::error::StackError::NotFound(
            "Delete branch not yet implemented".to_string(),
        ))
    }
}
