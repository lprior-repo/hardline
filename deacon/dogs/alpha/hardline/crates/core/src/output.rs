//! Output handling for CLI with verbosity control.
//!
//! Provides verbosity-aware output functions that respect --verbose and --quiet flags.

use std::sync::atomic::{AtomicU8, Ordering};

static VERBOSITY: AtomicU8 = AtomicU8::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Verbosity {
    Quiet = 0,
    #[default]
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

    // --- Verbosity: Default trait ---

    #[test]
    fn verbosity_default_is_normal() {
        let default = Verbosity::default();
        assert_eq!(default, Verbosity::Normal);
    }

    // --- Verbosity: is_quiet / is_verbose / is_normal ---

    #[test]
    fn verbosity_is_quiet() {
        assert!(Verbosity::Quiet.is_quiet());
        assert!(!Verbosity::Normal.is_quiet());
        assert!(!Verbosity::Verbose.is_quiet());
    }

    #[test]
    fn verbosity_is_verbose() {
        assert!(!Verbosity::Quiet.is_verbose());
        assert!(!Verbosity::Normal.is_verbose());
        assert!(Verbosity::Verbose.is_verbose());
    }

    #[test]
    fn verbosity_is_normal() {
        assert!(!Verbosity::Quiet.is_normal());
        assert!(Verbosity::Normal.is_normal());
        assert!(!Verbosity::Verbose.is_normal());
    }

    // --- Verbosity: discriminant values ---

    #[test]
    fn verbosity_discriminants() {
        assert_eq!(Verbosity::Quiet as u8, 0);
        assert_eq!(Verbosity::Normal as u8, 1);
        assert_eq!(Verbosity::Verbose as u8, 2);
    }

    // --- Verbosity: Copy and Clone ---

    #[test]
    fn verbosity_copy() {
        let v = Verbosity::Verbose;
        let copied = v;
        assert_eq!(v, copied);
    }

    #[test]
    fn verbosity_clone() {
        let v = Verbosity::Quiet;
        let cloned = v.clone();
        assert_eq!(v, cloned);
    }

    #[test]
    fn verbosity_debug() {
        let quiet = format!("{:?}", Verbosity::Quiet);
        let normal = format!("{:?}", Verbosity::Normal);
        let verbose = format!("{:?}", Verbosity::Verbose);
        assert!(quiet.contains("Quiet"));
        assert!(normal.contains("Normal"));
        assert!(verbose.contains("Verbose"));
    }

    #[test]
    fn verbosity_equality() {
        assert_eq!(Verbosity::Quiet, Verbosity::Quiet);
        assert_eq!(Verbosity::Normal, Verbosity::Normal);
        assert_eq!(Verbosity::Verbose, Verbosity::Verbose);
        assert_ne!(Verbosity::Quiet, Verbosity::Normal);
        assert_ne!(Verbosity::Normal, Verbosity::Verbose);
    }

    // --- Output: set_verbose and current_verbosity ---

    #[test]
    fn output_set_verbose_and_current_verbosity() {
        Output::set_verbose(true, false);
        assert_eq!(Output::current_verbosity(), Verbosity::Verbose);

        Output::set_verbose(false, true);
        assert_eq!(Output::current_verbosity(), Verbosity::Quiet);

        // Reset to default
        Output::set_verbose(false, false);
        assert_eq!(Output::current_verbosity(), Verbosity::Normal);
    }
}
