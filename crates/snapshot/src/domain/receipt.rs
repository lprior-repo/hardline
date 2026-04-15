//! Operation receipt data types.
//!
//! Receipts are stored as JSON files under `.git/stax/ops/<op-id>.json`

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Status of an operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OpStatus {
    InProgress,
    Success,
    Failed,
}

/// Kind of operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OpKind {
    Restack,
    UpstackRestack,
    SyncRestack,
    Submit,
    Reorder,
    Split,
    MergeWhenReady,
    Detach,
    Fix,
}

impl OpKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            OpKind::Restack => "restack",
            OpKind::UpstackRestack => "upstack restack",
            OpKind::SyncRestack => "sync --restack",
            OpKind::Submit => "submit",
            OpKind::Reorder => "reorder",
            OpKind::Split => "split",
            OpKind::MergeWhenReady => "merge-when-ready",
            OpKind::Detach => "detach",
            OpKind::Fix => "stack fix",
        }
    }
}

/// Information about a local ref that was modified
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalRefEntry {
    pub branch: String,
    pub refname: String,
    pub existed_before: bool,
    pub oid_before: Option<String>,
    pub oid_after: Option<String>,
}

/// Information about a remote ref that was modified (for submit)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRefEntry {
    pub remote: String,
    pub branch: String,
    pub remote_refname: String,
    pub oid_before: Option<String>,
    pub oid_after: Option<String>,
}

/// Error information for failed operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpError {
    pub message: String,
    pub failed_step: Option<String>,
    pub failed_branch: Option<String>,
}

/// Plan summary for display
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanSummary {
    pub branches_to_rebase: usize,
    pub branches_to_push: usize,
    pub description: Vec<String>,
}

/// Operation receipt - persisted to `.git/stax/ops/<op-id>.json`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpReceipt {
    pub op_id: String,
    pub kind: OpKind,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: OpStatus,
    pub repo_workdir: String,
    pub trunk: String,
    pub auto_stash_pop: bool,
    pub head_branch_before: String,
    pub local_refs: Vec<LocalRefEntry>,
    pub remote_refs: Vec<RemoteRefEntry>,
    pub plan_summary: PlanSummary,
    pub error: Option<OpError>,
}

impl OpReceipt {
    pub fn new(
        op_id: String,
        kind: OpKind,
        repo_workdir: String,
        trunk: String,
        head_branch_before: String,
    ) -> Self {
        let started_at = Utc::now().to_rfc3339();

        Self {
            op_id,
            kind,
            started_at,
            finished_at: None,
            status: OpStatus::InProgress,
            repo_workdir,
            trunk,
            auto_stash_pop: false,
            head_branch_before,
            local_refs: Vec::new(),
            remote_refs: Vec::new(),
            plan_summary: PlanSummary::default(),
            error: None,
        }
    }

    pub fn add_local_ref(&mut self, branch: &str, oid_before: Option<&str>) {
        self.local_refs.push(LocalRefEntry {
            branch: branch.to_string(),
            refname: format!("refs/heads/{}", branch),
            existed_before: oid_before.is_some(),
            oid_before: oid_before.map(String::from),
            oid_after: None,
        });
    }

    pub fn add_remote_ref(&mut self, remote: &str, branch: &str, oid_before: Option<&str>) {
        self.remote_refs.push(RemoteRefEntry {
            remote: remote.to_string(),
            branch: branch.to_string(),
            remote_refname: format!("refs/remotes/{}/{}", remote, branch),
            oid_before: oid_before.map(String::from),
            oid_after: None,
        });
    }

    pub fn update_local_ref_after(&mut self, branch: &str, oid_after: &str) {
        if let Some(entry) = self.local_refs.iter_mut().find(|e| e.branch == branch) {
            entry.oid_after = Some(oid_after.to_string());
        }
    }

    pub fn update_remote_ref_after(&mut self, remote: &str, branch: &str, oid_after: &str) {
        if let Some(entry) = self
            .remote_refs
            .iter_mut()
            .find(|e| e.remote == remote && e.branch == branch)
        {
            entry.oid_after = Some(oid_after.to_string());
        }
    }

    pub fn mark_success(&mut self) {
        self.status = OpStatus::Success;
        self.finished_at = Some(Utc::now().to_rfc3339());
    }

    pub fn mark_failed(
        &mut self,
        message: &str,
        failed_step: Option<&str>,
        failed_branch: Option<&str>,
    ) {
        self.status = OpStatus::Failed;
        self.finished_at = Some(Utc::now().to_rfc3339());
        self.error = Some(OpError {
            message: message.to_string(),
            failed_step: failed_step.map(String::from),
            failed_branch: failed_branch.map(String::from),
        });
    }
}
