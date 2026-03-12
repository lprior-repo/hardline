//! Cleanup and rollback handling for pipeline phases

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::state::{PipelineId, PipelineState};

/// Types of phases that can be cleaned up
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseType {
    SpecReview,
    UniverseSetup,
    AgentDevelopment,
    Validation,
}

impl PhaseType {
    /// Convert from PipelineState to PhaseType
    #[must_use]
    pub fn from_state(state: PipelineState) -> Option<Self> {
        match state {
            PipelineState::SpecReview => Some(Self::SpecReview),
            PipelineState::UniverseSetup => Some(Self::UniverseSetup),
            PipelineState::AgentDevelopment => Some(Self::AgentDevelopment),
            PipelineState::Validation => Some(Self::Validation),
            _ => None,
        }
    }
}

/// Resource identifier for tracking created resources
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceId(pub String);

impl ResourceId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Context for cleanup operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupContext {
    pub pipeline_id: PipelineId,
    pub failed_phase: PhaseType,
    pub created_resources: Vec<ResourceId>,
    pub rollback_data: Vec<u8>,
}

impl CleanupContext {
    #[must_use]
    pub fn new(pipeline_id: PipelineId, failed_phase: PhaseType) -> Self {
        Self {
            pipeline_id,
            failed_phase,
            created_resources: Vec::new(),
            rollback_data: Vec::new(),
        }
    }

    pub fn add_resource(&mut self, resource: ResourceId) {
        self.created_resources.push(resource);
    }

    pub fn set_rollback_data(&mut self, data: Vec<u8>) {
        self.rollback_data = data;
    }
}

/// Errors that can occur during cleanup
#[derive(Debug, Clone, Error)]
pub enum CleanupError {
    #[error("Cleanup not implemented for phase: {0}")]
    NotImplemented(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Cleanup failed: {0}")]
    CleanupFailed(String),

    #[error("Rollback failed: {0}")]
    RollbackFailed(String),
}

/// Status of cleanup operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupStatus {
    Success,
    Failed(Vec<String>),
}

/// Result of cleanup operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupResult {
    pub status: CleanupStatus,
    pub cleaned_resources: Vec<ResourceId>,
}

impl CleanupResult {
    #[must_use]
    pub fn success() -> Self {
        Self {
            status: CleanupStatus::Success,
            cleaned_resources: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_resource(mut self, resource: ResourceId) -> Self {
        self.cleaned_resources.push(resource);
        self
    }

    #[must_use]
    pub fn with_error(mut self, error: String) -> Self {
        let errors = match &mut self.status {
            CleanupStatus::Success => {
                vec![error]
            }
            CleanupStatus::Failed(errs) => {
                errs.push(error);
                errs.clone()
            }
        };
        self.status = CleanupStatus::Failed(errors);
        self
    }

    #[must_use]
    pub fn success_flag(&self) -> bool {
        matches!(self.status, CleanupStatus::Success)
    }

    #[must_use]
    pub fn errors(&self) -> Vec<String> {
        match &self.status {
            CleanupStatus::Success => Vec::new(),
            CleanupStatus::Failed(errs) => errs.clone(),
        }
    }
}

/// Trait for cleanup handlers per phase
pub trait CleanupHandler: Send + Sync {
    /// Returns the phase type this handler manages
    fn phase_type(&self) -> PhaseType;

    /// Execute cleanup for the given context
    fn cleanup(&self, context: &CleanupContext) -> CleanupResult;

    /// Execute rollback using saved rollback data
    fn rollback(&self, context: &CleanupContext) -> CleanupResult;
}

/// No-op cleanup handler for phases that don't need cleanup
pub struct NoopCleanupHandler;

impl CleanupHandler for NoopCleanupHandler {
    fn phase_type(&self) -> PhaseType {
        PhaseType::SpecReview
    }

    fn cleanup(&self, _context: &CleanupContext) -> CleanupResult {
        CleanupResult::success()
    }

    fn rollback(&self, _context: &CleanupContext) -> CleanupResult {
        CleanupResult::success()
    }
}

/// Cleanup handler for universe setup phase
pub struct UniverseSetupCleanupHandler;

impl CleanupHandler for UniverseSetupCleanupHandler {
    fn phase_type(&self) -> PhaseType {
        PhaseType::UniverseSetup
    }

    fn cleanup(&self, context: &CleanupContext) -> CleanupResult {
        // In a real implementation, this would:
        // 1. Delete temporary files created during setup
        // 2. Kill spawned processes
        // 3. Release network resources
        let mut result = CleanupResult::success();

        for resource in &context.created_resources {
            // Placeholder: In production, actually clean up resources
            result = result.with_resource(resource.clone());
        }

        result
    }

    fn rollback(&self, context: &CleanupContext) -> CleanupResult {
        // In a real implementation, this would:
        // 1. Parse rollback data
        // 2. Restore previous state
        // 3. Undo configuration changes
        let mut result = CleanupResult::success();

        if !context.rollback_data.is_empty() {
            // Placeholder: In production, actually rollback
            result = result.with_error("Rollback data present but not implemented".to_string());
        }

        result
    }
}

/// Cleanup handler for agent development phase
pub struct AgentDevelopmentCleanupHandler;

impl CleanupHandler for AgentDevelopmentCleanupHandler {
    fn phase_type(&self) -> PhaseType {
        PhaseType::AgentDevelopment
    }

    fn cleanup(&self, context: &CleanupContext) -> CleanupResult {
        // In a real implementation, this would:
        // 1. Kill spawned agent processes
        // 2. Clean up temporary workspaces
        // 3. Release any locks
        let mut result = CleanupResult::success();

        for resource in &context.created_resources {
            result = result.with_resource(resource.clone());
        }

        result
    }

    fn rollback(&self, context: &CleanupContext) -> CleanupResult {
        // In a real implementation, this would:
        // 1. Revert code changes
        // 2. Restore previous iteration state
        let mut result = CleanupResult::success();

        if !context.rollback_data.is_empty() {
            result = result.with_error("Rollback data present but not implemented".to_string());
        }

        result
    }
}

/// Manages cleanup handlers for all phase types
pub struct CleanupManager {
    handlers: std::collections::HashMap<PhaseType, Box<dyn CleanupHandler>>,
}

impl CleanupManager {
    #[must_use]
    pub fn new() -> Self {
        let mut handlers: std::collections::HashMap<PhaseType, Box<dyn CleanupHandler>> =
            std::collections::HashMap::new();

        // Register default noop handlers
        handlers.insert(PhaseType::SpecReview, Box::new(NoopCleanupHandler));
        handlers.insert(
            PhaseType::UniverseSetup,
            Box::new(UniverseSetupCleanupHandler),
        );
        handlers.insert(
            PhaseType::AgentDevelopment,
            Box::new(AgentDevelopmentCleanupHandler),
        );
        handlers.insert(PhaseType::Validation, Box::new(NoopCleanupHandler));

        Self { handlers }
    }

    pub fn register_handler(&mut self, handler: Box<dyn CleanupHandler>) {
        self.handlers.insert(handler.phase_type(), handler);
    }

    #[must_use]
    pub fn get_handler(&self, phase: PhaseType) -> Option<&dyn CleanupHandler> {
        self.handlers.get(&phase).map(|h| h.as_ref())
    }

    pub fn cleanup(&self, context: &CleanupContext) -> CleanupResult {
        let handler = self.get_handler(context.failed_phase);

        match handler {
            Some(h) => h.cleanup(context),
            None => CleanupResult::success(),
        }
    }

    pub fn rollback(&self, context: &CleanupContext) -> CleanupResult {
        let handler = self.get_handler(context.failed_phase);

        match handler {
            Some(h) => h.rollback(context),
            None => CleanupResult::success(),
        }
    }
}

impl Default for CleanupManager {
    fn default() -> Self {
        Self::new()
    }
}
