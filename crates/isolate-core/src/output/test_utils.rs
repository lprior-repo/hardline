//! Test utilities for capturing `OutputLine` emissions
//!
//! This module provides types for dependency injection of output emission,
//! enabling both production (stdout) and test (in-memory capture) scenarios.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::{io, sync::RwLock};

use super::OutputLine;

/// Trait for dependency injection of output emission.
///
/// This trait enables abstracting over output destinations, allowing
/// production code to write to stdout while tests capture to memory.
pub trait OutputEmitter: Send + Sync {
    /// Emit a single output line.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the emission fails.
    fn emit(&self, line: &OutputLine) -> io::Result<()>;
}

/// Production implementation that writes to stdout.
///
/// This struct delegates to [`super::emit_stdout`] for actual output.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdoutEmitter;

impl StdoutEmitter {
    /// Create a new stdout emitter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl OutputEmitter for StdoutEmitter {
    fn emit(&self, line: &OutputLine) -> io::Result<()> {
        super::emit_stdout(line)
    }
}

/// Test implementation that captures lines in a `Vec`.
///
/// Uses interior mutability via `RwLock` to allow capturing in
/// multi-threaded test contexts without requiring `&mut self`.
///
/// # Example
///
/// ```
/// use isolate_core::output::{
///     domain_types::Message, OutputEmitter, OutputLine, Summary, SummaryType, VecEmitter,
/// };
///
/// let emitter = VecEmitter::new();
/// let message = Message::new("test").unwrap();
/// let summary = Summary::new(SummaryType::Info, message).unwrap();
///
/// emitter.emit(&OutputLine::Summary(summary)).unwrap();
///
/// let lines = emitter.take_lines();
/// assert_eq!(lines.len(), 1);
/// ```
#[derive(Debug, Default)]
pub struct VecEmitter(pub RwLock<Vec<OutputLine>>);

impl VecEmitter {
    /// Create a new empty emitter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Remove and return all captured lines.
    ///
    /// After calling this method, the emitter will be empty.
    #[must_use]
    pub fn take_lines(&self) -> Vec<OutputLine> {
        self.0
            .write()
            .map_or_else(|_| Vec::new(), |mut guard| guard.drain(..).collect())
    }

    /// Get a clone of all captured lines without removing them.
    #[must_use]
    pub fn peek_lines(&self) -> Vec<OutputLine> {
        self.0
            .read()
            .map_or_else(|_| Vec::new(), |guard| guard.clone())
    }

    /// Get the number of captured lines.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.read().map_or(0, |guard| guard.len())
    }

    /// Check if no lines have been captured.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.read().map_or(true, |guard| guard.is_empty())
    }
}

impl OutputEmitter for VecEmitter {
    fn emit(&self, line: &OutputLine) -> io::Result<()> {
        self.0
            .write()
            .map_err(|_| io::Error::other("VecEmitter lock poisoned"))?
            .push(line.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::{domain_types::Message, Summary, SummaryType};

    #[test]
    fn vec_emitter_captures_lines() -> Result<(), Box<dyn std::error::Error>> {
        let emitter = VecEmitter::new();
        let summary = Summary::new(SummaryType::Info, Message::new("test message")?)?;

        assert!(emitter.is_empty());

        emitter.emit(&OutputLine::Summary(summary))?;

        assert_eq!(emitter.len(), 1);
        assert!(!emitter.is_empty());

        let lines = emitter.peek_lines();
        assert_eq!(lines.len(), 1);

        let taken = emitter.take_lines();
        assert_eq!(taken.len(), 1);

        assert!(emitter.is_empty());
        Ok(())
    }

    #[test]
    fn vec_emitter_accumulates_multiple_lines() -> Result<(), Box<dyn std::error::Error>> {
        let emitter = VecEmitter::new();

        for i in 0..3 {
            let summary = Summary::new(SummaryType::Info, Message::new(format!("message {i}"))?)?;
            emitter.emit(&OutputLine::Summary(summary))?;
        }

        assert_eq!(emitter.len(), 3);

        let lines = emitter.take_lines();
        assert_eq!(lines.len(), 3);
        assert!(emitter.is_empty());
        Ok(())
    }
}
