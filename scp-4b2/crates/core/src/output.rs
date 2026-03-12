//! Output handling for CLI with verbosity control.
//!
//! Provides verbosity-aware output functions that respect --verbose and --quiet flags.

use std::sync::atomic::{AtomicU8, Ordering};

static VERBOSITY: AtomicU8 = AtomicU8::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Quiet = 0,
    Normal = 1,
    Verbose = 2,
}

impl Verbosity {
    pub fn current() -> Self {
        match VERBOSITY.load(Ordering::SeqCst) {
            0 => Self::Quiet,
            1 => Self::Normal,
            _ => Self::Verbose,
        }
    }

    pub fn set(verbose: bool, quiet: bool) {
        let level = match (verbose, quiet) {
            (true, false) => 2,
            (false, true) => 0,
            (true, true) => 0,
            _ => 1,
        };
        VERBOSITY.store(level, Ordering::SeqCst);
    }

    #[must_use]
    pub fn is_quiet(self) -> bool {
        self == Self::Quiet
    }

    #[must_use]
    pub fn is_verbose(self) -> bool {
        self == Self::Verbose
    }

    #[must_use]
    pub fn is_normal(self) -> bool {
        self == Self::Normal
    }
}

impl Default for Verbosity {
    fn default() -> Self {
        Self::Normal
    }
}

pub struct Output;

impl Output {
    pub fn verbose(msg: &str) {
        if Verbosity::current().is_verbose() {
            println!("[verbose] {}", msg);
        }
    }

    pub fn info(msg: &str) {
        if !Verbosity::current().is_quiet() {
            println!("{}", msg);
        }
    }

    pub fn success(msg: &str) {
        if !Verbosity::current().is_quiet() {
            println!("✓ {}", msg);
        }
    }

    pub fn error(msg: &str) {
        eprintln!("✗ {}", msg);
    }

    pub fn warn(msg: &str) {
        if !Verbosity::current().is_quiet() {
            eprintln!("⚠ {}", msg);
        }
    }

    pub fn step(step: usize, total: usize, msg: &str) {
        if !Verbosity::current().is_quiet() {
            println!("[{}/{}] {}", step, total, msg);
        }
    }

    pub fn debug(msg: &str) {
        if Verbosity::current().is_verbose() {
            eprintln!("[debug] {}", msg);
        }
    }

    pub fn set_verbose(verbose: bool, quiet: bool) {
        Verbosity::set(verbose, quiet);
    }

    pub fn current_verbosity() -> Verbosity {
        Verbosity::current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verbosity_default() {
        Verbosity::set(false, false);
        assert_eq!(Verbosity::current(), Verbosity::Normal);
    }

    #[test]
    fn test_verbosity_quiet() {
        Verbosity::set(false, true);
        assert_eq!(Verbosity::current(), Verbosity::Quiet);
    }

    #[test]
    fn test_verbosity_verbose() {
        Verbosity::set(true, false);
        assert_eq!(Verbosity::current(), Verbosity::Verbose);
    }

    #[test]
    fn test_verbosity_quiet_overrides_verbose() {
        Verbosity::set(true, true);
        assert_eq!(Verbosity::current(), Verbosity::Quiet);
    }
}
