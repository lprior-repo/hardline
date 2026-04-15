use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiRunRecord {
    pub run_id: String,
    pub commit_sha: String,
    pub status: CiStatus,
    pub started_at: DateTime<Utc>,
    pub duration_secs: u64,
}

impl CiRunRecord {
    pub fn new(
        run_id: impl Into<String>,
        commit_sha: impl Into<String>,
        status: CiStatus,
        started_at: DateTime<Utc>,
        duration_secs: u64,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            commit_sha: commit_sha.into(),
            status,
            started_at,
            duration_secs,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CiStatus {
    Success,
    Failure,
    Pending,
    InProgress,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiCheckHistory {
    pub check_name: String,
    pub branch_name: String,
    pub records: Vec<CiRunRecord>,
}

impl CiCheckHistory {
    pub fn new(check_name: impl Into<String>, branch_name: impl Into<String>) -> Self {
        Self {
            check_name: check_name.into(),
            branch_name: branch_name.into(),
            records: Vec::new(),
        }
    }

    pub fn with_records(
        check_name: impl Into<String>,
        branch_name: impl Into<String>,
        records: Vec<CiRunRecord>,
    ) -> Self {
        Self {
            check_name: check_name.into(),
            branch_name: branch_name.into(),
            records,
        }
    }

    pub fn average_duration_secs(&self) -> Option<f64> {
        if self.records.is_empty() {
            return None;
        }
        let total: u64 = self.records.iter().map(|r| r.duration_secs).sum();
        Some(total as f64 / self.records.len() as f64)
    }

    pub fn last_run(&self) -> Option<&CiRunRecord> {
        self.records.last()
    }

    pub fn add_record(&mut self, record: CiRunRecord) {
        self.records.push(record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ci_run_record_new() {
        let record = CiRunRecord::new("run-123", "abc123", CiStatus::Success, Utc::now(), 120);
        assert_eq!(record.run_id, "run-123");
        assert_eq!(record.commit_sha, "abc123");
        assert_eq!(record.status, CiStatus::Success);
        assert_eq!(record.duration_secs, 120);
    }

    #[test]
    fn test_ci_status_variants() {
        let statuses = vec![
            CiStatus::Success,
            CiStatus::Failure,
            CiStatus::Pending,
            CiStatus::InProgress,
            CiStatus::Cancelled,
        ];
        for status in statuses {
            assert_ne!(format!("{status:?}"), "");
        }
    }

    #[test]
    fn test_ci_check_history_new() {
        let history = CiCheckHistory::new("check", "main");
        assert_eq!(history.check_name, "check");
        assert_eq!(history.branch_name, "main");
        assert!(history.records.is_empty());
    }

    #[test]
    fn test_ci_check_history_average_empty() {
        let history = CiCheckHistory::new("check", "main");
        assert!(history.average_duration_secs().is_none());
    }

    #[test]
    fn test_ci_check_history_average() {
        let mut history = CiCheckHistory::new("check", "main");
        history.add_record(CiRunRecord::new(
            "r1",
            "c1",
            CiStatus::Success,
            Utc::now(),
            100,
        ));
        history.add_record(CiRunRecord::new(
            "r2",
            "c2",
            CiStatus::Success,
            Utc::now(),
            200,
        ));
        assert_eq!(history.average_duration_secs(), Some(150.0));
    }

    #[test]
    fn test_ci_check_history_last_run() {
        let mut history = CiCheckHistory::new("check", "main");
        assert!(history.last_run().is_none());
        history.add_record(CiRunRecord::new(
            "r1",
            "c1",
            CiStatus::Success,
            Utc::now(),
            100,
        ));
        history.add_record(CiRunRecord::new(
            "r2",
            "c2",
            CiStatus::Failure,
            Utc::now(),
            150,
        ));
        assert_eq!(history.last_run().map(|r| r.run_id.as_str()), Some("r2"));
    }

    #[test]
    fn test_ci_check_history_add_record() {
        let mut history = CiCheckHistory::new("check", "main");
        assert_eq!(history.records.len(), 0);
        history.add_record(CiRunRecord::new(
            "r1",
            "c1",
            CiStatus::Success,
            Utc::now(),
            100,
        ));
        assert_eq!(history.records.len(), 1);
    }

    #[test]
    fn test_ci_run_record_serde() {
        let record = CiRunRecord::new("run-123", "abc123", CiStatus::Success, Utc::now(), 120);
        let json = serde_json::to_string(&record).expect("serialize");
        let deserialized: CiRunRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(record, deserialized);
    }

    #[test]
    fn test_ci_check_history_serde() {
        let mut history = CiCheckHistory::new("check", "main");
        history.add_record(CiRunRecord::new(
            "r1",
            "c1",
            CiStatus::Success,
            Utc::now(),
            100,
        ));
        let json = serde_json::to_string(&history).expect("serialize");
        let deserialized: CiCheckHistory = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(history, deserialized);
    }
}
