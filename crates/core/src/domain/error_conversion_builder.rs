//! Builder error conversion implementations.
//!
//! This module provides `From<BuilderError>` implementations
//! for aggregate errors and repository errors.

use crate::domain::{
    aggregates::{bead::BeadError, session::SessionError, workspace::WorkspaceError},
    builders::BuilderError,
    repository::RepositoryError,
};

impl From<BuilderError> for SessionError {
    fn from(err: BuilderError) -> Self {
        match &err {
            BuilderError::MissingRequired { field: _ }
            | BuilderError::InvalidValue {
                field: _,
                reason: _,
            }
            | BuilderError::Overflow {
                field: _,
                capacity: _,
            }
            | BuilderError::InvalidTransition {
                from: _,
                to: _,
                reason: _,
            } => Self::CannotActivate,
        }
    }
}

impl From<BuilderError> for WorkspaceError {
    fn from(err: BuilderError) -> Self {
        match &err {
            BuilderError::MissingRequired { .. }
            | BuilderError::InvalidValue { .. }
            | BuilderError::Overflow { .. }
            | BuilderError::InvalidTransition { .. } => {
                Self::CannotUse(crate::domain::workspace::WorkspaceState::Creating)
            }
        }
    }
}

impl From<BuilderError> for BeadError {
    fn from(err: BuilderError) -> Self {
        match &err {
            BuilderError::MissingRequired { field } => match *field {
                "title" => Self::TitleRequired,
                _ => Self::InvalidTitle(format!("missing required field: {field}")),
            },
            BuilderError::InvalidValue { field, reason } => {
                if *field == "title" {
                    Self::InvalidTitle(reason.clone())
                } else {
                    Self::InvalidTitle(format!("invalid {field}: {reason}"))
                }
            }
            BuilderError::Overflow { field, capacity } => {
                Self::InvalidTitle(format!("field {field} exceeds capacity {capacity}"))
            }
            BuilderError::InvalidTransition { from, to, reason } => {
                Self::InvalidTitle(format!("invalid transition from {from} to {to}: {reason}"))
            }
        }
    }
}

impl From<BuilderError> for RepositoryError {
    fn from(err: BuilderError) -> Self {
        match &err {
            BuilderError::MissingRequired { field } => {
                Self::InvalidInput(format!("missing required field: {field}"))
            }
            BuilderError::InvalidValue { field, reason } => {
                Self::InvalidInput(format!("invalid value for {field}: {reason}"))
            }
            BuilderError::Overflow { field, capacity } => {
                Self::InvalidInput(format!("field {field} exceeds capacity of {capacity}"))
            }
            BuilderError::InvalidTransition { from, to, reason } => {
                Self::InvalidInput(format!("invalid transition from {from} to {to}: {reason}",))
            }
        }
    }
}
