use crate::domain::entities::Stack;
use crate::error::{Result, StackError};

const OPERATION_NOT_IMPLEMENTED: &str = "not yet implemented";

pub struct StackEngine;

impl StackEngine {
    #[must_use]
    pub fn load_stack() -> Result<Stack> {
        not_found_error("Stack loading")
    }

    #[must_use]
    pub fn sync_stack() -> Result<Stack> {
        not_found_error("Stack sync")
    }

    #[must_use]
    pub fn restack_branch(_branch: &str) -> Result<()> {
        not_found_error_void("Restack")
    }

    #[must_use]
    pub fn create_branch(_name: &str, _parent: Option<&str>) -> Result<()> {
        not_found_error_void("Create branch")
    }

    #[must_use]
    pub fn delete_branch(_name: &str) -> Result<()> {
        not_found_error_void("Delete branch")
    }
}

const fn not_found_error(operation: &str) -> Result<Stack> {
    Err(StackError::NotFound(format!(
        "{operation} {OPERATION_NOT_IMPLEMENTED}"
    )))
}

fn not_found_error_void(operation: &str) -> Result<()> {
    Err(StackError::NotFound(format!(
        "{operation} {OPERATION_NOT_IMPLEMENTED}"
    )))
}
