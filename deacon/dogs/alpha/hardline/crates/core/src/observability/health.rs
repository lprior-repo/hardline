//! Health checks

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
}

impl HealthState {
    #[must_use]
    pub fn worst(a: HealthState, b: HealthState) -> HealthState {
        match (a, b) {
            (HealthState::Unhealthy, _) => HealthState::Unhealthy,
            (_, HealthState::Unhealthy) => HealthState::Unhealthy,
            (HealthState::Degraded, _) => HealthState::Degraded,
            (_, HealthState::Degraded) => HealthState::Degraded,
            _ => HealthState::Healthy,
        }
    }
}

impl std::fmt::Display for HealthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthState::Healthy => write!(f, "Healthy"),
            HealthState::Degraded => write!(f, "Degraded"),
            HealthState::Unhealthy => write!(f, "Unhealthy"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthState,
    pub message: Option<String>,
    pub latency_ms: Option<u64>,
}

impl HealthCheck {
    #[must_use]
    pub fn healthy(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthState::Healthy,
            message: None,
            latency_ms: None,
        }
    }

    #[must_use]
    pub fn degraded(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthState::Degraded,
            message: Some(message.into()),
            latency_ms: None,
        }
    }

    #[must_use]
    pub fn unhealthy(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthState::Unhealthy,
            message: Some(message.into()),
            latency_ms: None,
        }
    }

    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = Some(latency_ms);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: HealthState,
    pub checks: Vec<HealthCheck>,
    pub timestamp: DateTime<Utc>,
}

impl HealthStatus {
    #[must_use]
    pub fn new(checks: Vec<HealthCheck>) -> Self {
        let status = checks
            .iter()
            .map(|c| c.status)
            .fold(HealthState::Healthy, HealthState::worst);

        Self {
            status,
            checks,
            timestamp: Utc::now(),
        }
    }

    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.status == HealthState::Healthy
    }
}

pub trait HealthCheckerTrait: Send + Sync {
    fn check_name(&self) -> &str;
    fn perform_check(&self) -> HealthCheck;
}

pub struct HealthChecker {
    checks: Vec<Box<dyn HealthCheckerTrait>>,
}

impl HealthChecker {
    #[must_use]
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    pub fn add_check<C>(mut self, checker: C) -> Self
    where
        C: HealthCheckerTrait + 'static,
    {
        self.checks.push(Box::new(checker));
        self
    }

    pub fn check_all(&self) -> HealthStatus {
        let checks: Vec<HealthCheck> = self.checks.iter().map(|c| c.perform_check()).collect();
        HealthStatus::new(checks)
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DatabaseHealthCheck;

impl HealthCheckerTrait for DatabaseHealthCheck {
    fn check_name(&self) -> &str {
        "database"
    }

    fn perform_check(&self) -> HealthCheck {
        HealthCheck::healthy("database")
    }
}

pub struct VcsHealthCheck;

impl HealthCheckerTrait for VcsHealthCheck {
    fn check_name(&self) -> &str {
        "vcs"
    }

    fn perform_check(&self) -> HealthCheck {
        HealthCheck::healthy("vcs")
    }
}

pub struct DiskSpaceCheck;

impl HealthCheckerTrait for DiskSpaceCheck {
    fn check_name(&self) -> &str {
        "disk_space"
    }

    fn perform_check(&self) -> HealthCheck {
        HealthCheck::healthy("disk_space")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_state_worst() {
        assert_eq!(
            HealthState::worst(HealthState::Healthy, HealthState::Healthy),
            HealthState::Healthy
        );
        assert_eq!(
            HealthState::worst(HealthState::Healthy, HealthState::Degraded),
            HealthState::Degraded
        );
        assert_eq!(
            HealthState::worst(HealthState::Degraded, HealthState::Unhealthy),
            HealthState::Unhealthy
        );
        assert_eq!(
            HealthState::worst(HealthState::Unhealthy, HealthState::Healthy),
            HealthState::Unhealthy
        );
    }

    #[test]
    fn test_health_check_healthy() {
        let check = HealthCheck::healthy("test");
        assert_eq!(check.name, "test");
        assert_eq!(check.status, HealthState::Healthy);
        assert!(check.message.is_none());
        assert!(check.latency_ms.is_none());
    }

    #[test]
    fn test_health_check_with_latency() {
        let check = HealthCheck::healthy("test").with_latency(100);
        assert_eq!(check.latency_ms, Some(100));
    }

    #[test]
    fn test_health_status_from_checks() {
        let checks = vec![
            HealthCheck::healthy("check1"),
            HealthCheck::degraded("check2", "something is off"),
        ];
        let status = HealthStatus::new(checks);

        assert_eq!(status.status, HealthState::Degraded);
        assert_eq!(status.checks.len(), 2);
    }

    #[test]
    fn test_health_status_all_healthy() {
        let checks = vec![
            HealthCheck::healthy("check1"),
            HealthCheck::healthy("check2"),
        ];
        let status = HealthStatus::new(checks);

        assert!(status.is_healthy());
    }

    #[test]
    fn test_health_checker_check_all() {
        let checker = HealthChecker::new()
            .add_check(DatabaseHealthCheck)
            .add_check(VcsHealthCheck)
            .add_check(DiskSpaceCheck);

        let status = checker.check_all();

        assert_eq!(status.checks.len(), 3);
        assert!(status.is_healthy());
    }

    #[test]
    fn test_health_state_display() {
        assert_eq!(format!("{}", HealthState::Healthy), "Healthy");
        assert_eq!(format!("{}", HealthState::Degraded), "Degraded");
        assert_eq!(format!("{}", HealthState::Unhealthy), "Unhealthy");
    }
}
