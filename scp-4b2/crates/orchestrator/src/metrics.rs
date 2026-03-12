//! Metrics collection for pipeline execution

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use im::Vector;
use serde::{Deserialize, Serialize};

/// A single scenario test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub name: String,
    pub passed: bool,
    pub duration_secs: f64,
    pub error: Option<String>,
}

/// Metrics for a single phase execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseMetrics {
    pub pipeline_id: String,
    pub phase: String,
    pub started_at: DateTime<Utc>,
    pub duration_secs: f64,
    pub success: bool,
}

/// Complete metrics for a pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineMetrics {
    pub pipeline_id: String,
    pub total_duration_secs: f64,
    pub phase_metrics: Vec<PhaseMetrics>,
    pub iteration_count: u32,
    pub scenario_results: Vec<ScenarioResult>,
    pub final_state: String,
}

/// Aggregated metrics across all pipelines
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    pub total_pipelines: u32,
    pub successful_pipelines: u32,
    pub failed_pipelines: u32,
    pub escalated_pipelines: u32,
    pub average_duration_secs: f64,
    pub average_iterations: f64,
    pub phase_durations: HashMap<String, f64>,
}

/// Metrics collector
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metrics {
    #[serde(skip)]
    phase_metrics: Vector<PhaseMetrics>,
    #[serde(skip)]
    pipeline_metrics: HashMap<String, PipelineMetrics>,
}

impl Metrics {
    #[must_use]
    pub fn new() -> Self {
        Self {
            phase_metrics: Vector::new(),
            pipeline_metrics: HashMap::new(),
        }
    }

    pub fn record_phase(&mut self, metrics: PhaseMetrics) {
        let pipeline_id = metrics.pipeline_id.clone();
        let entry = self
            .pipeline_metrics
            .entry(pipeline_id.clone())
            .or_insert_with(|| PipelineMetrics {
                pipeline_id,
                total_duration_secs: 0.0,
                phase_metrics: vec![],
                iteration_count: 0,
                scenario_results: vec![],
                final_state: "unknown".to_string(),
            });

        entry.total_duration_secs += metrics.duration_secs;
        entry.phase_metrics.push(metrics.clone());

        self.phase_metrics.push_back(metrics);
    }

    pub fn record_scenarios(&mut self, pipeline_id: &str, results: Vec<ScenarioResult>) {
        if let Some(metrics) = self.pipeline_metrics.get_mut(pipeline_id) {
            metrics.scenario_results = results;
        }
    }

    pub fn record_iteration(&mut self, pipeline_id: &str, count: u32) {
        if let Some(metrics) = self.pipeline_metrics.get_mut(pipeline_id) {
            metrics.iteration_count = count;
        }
    }

    pub fn mark_complete(&mut self, pipeline_id: &str, final_state: &str) {
        if let Some(metrics) = self.pipeline_metrics.get_mut(pipeline_id) {
            metrics.final_state = final_state.to_string();
        }
    }

    #[must_use]
    pub fn get_pipeline_metrics(&self, pipeline_id: &str) -> Option<&PipelineMetrics> {
        self.pipeline_metrics.get(pipeline_id)
    }

    pub fn get_phase_metrics(&self) -> impl Iterator<Item = &PhaseMetrics> {
        self.phase_metrics.iter()
    }

    #[must_use]
    pub fn get_for_pipeline(&self, pipeline_id: &str) -> Vec<&PhaseMetrics> {
        self.phase_metrics
            .iter()
            .filter(|m| m.pipeline_id == pipeline_id)
            .collect()
    }

    #[must_use]
    pub fn aggregated(&self) -> AggregatedMetrics {
        let pipelines: Vec<_> = self.pipeline_metrics.values().collect();

        if pipelines.is_empty() {
            return AggregatedMetrics::default();
        }

        let total = u32::try_from(pipelines.len()).unwrap_or(u32::MAX);
        let successful = u32::try_from(
            pipelines
                .iter()
                .filter(|p| p.final_state == "accepted")
                .count(),
        )
        .unwrap_or(u32::MAX);
        let failed = u32::try_from(
            pipelines
                .iter()
                .filter(|p| p.final_state == "failed")
                .count(),
        )
        .unwrap_or(u32::MAX);
        let escalated = u32::try_from(
            pipelines
                .iter()
                .filter(|p| p.final_state == "escalated")
                .count(),
        )
        .unwrap_or(u32::MAX);

        let total_duration: f64 = pipelines.iter().map(|p| p.total_duration_secs).sum();
        let total_iterations: u32 = pipelines.iter().map(|p| p.iteration_count).sum();

        let average_duration = total_duration / f64::from(total);
        let average_iterations = f64::from(total_iterations) / f64::from(total);

        let mut phase_durations: HashMap<String, Vec<f64>> = HashMap::new();
        for pipeline in &pipelines {
            for phase in &pipeline.phase_metrics {
                phase_durations
                    .entry(phase.phase.clone())
                    .or_default()
                    .push(phase.duration_secs);
            }
        }

        let phase_durations: HashMap<String, f64> = phase_durations
            .into_iter()
            .map(|(k, v)| {
                let sum: f64 = v.iter().sum();
                let len = v.len() as f64;
                (k, sum / len)
            })
            .collect();

        AggregatedMetrics {
            total_pipelines: total,
            successful_pipelines: successful,
            failed_pipelines: failed,
            escalated_pipelines: escalated,
            average_duration_secs: average_duration,
            average_iterations,
            phase_durations,
        }
    }

    #[must_use]
    pub fn success_rate(&self) -> f64 {
        let total = self.pipeline_metrics.len();
        if total == 0 {
            return 0.0;
        }

        let successful = self
            .pipeline_metrics
            .values()
            .filter(|p| p.final_state == "accepted")
            .count();

        (successful as f64 / total as f64) * 100.0
    }

    #[must_use]
    pub fn scenario_pass_rate(&self) -> f64 {
        let all_results: Vec<_> = self
            .pipeline_metrics
            .values()
            .flat_map(|p| p.scenario_results.iter())
            .collect();

        if all_results.is_empty() {
            return 0.0;
        }

        let passed = all_results.iter().filter(|r| r.passed).count();
        (passed as f64 / all_results.len() as f64) * 100.0
    }

    #[must_use]
    pub fn slowest_phases(&self, limit: usize) -> Vec<(String, f64)> {
        let mut phase_totals: HashMap<String, f64> = HashMap::new();

        for metrics in &self.phase_metrics {
            *phase_totals.entry(metrics.phase.clone()).or_insert(0.0) += metrics.duration_secs;
        }

        let mut phases: Vec<_> = phase_totals.into_iter().collect();
        phases.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        phases.into_iter().take(limit).collect()
    }

    #[cfg(test)]
    pub fn clear(&mut self) {
        self.phase_metrics.clear();
        self.pipeline_metrics.clear();
    }

    pub fn export(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.pipeline_metrics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_phase() {
        let mut metrics = Metrics::new();

        metrics.record_phase(PhaseMetrics {
            pipeline_id: "test-1".to_string(),
            phase: "spec_review".to_string(),
            started_at: Utc::now(),
            duration_secs: 1.5,
            success: true,
        });

        assert_eq!(metrics.pipeline_metrics.len(), 1);
    }

    #[test]
    fn test_aggregated_metrics() {
        let mut metrics = Metrics::new();

        metrics.record_phase(PhaseMetrics {
            pipeline_id: "test-1".to_string(),
            phase: "spec_review".to_string(),
            started_at: Utc::now(),
            duration_secs: 1.5,
            success: true,
        });

        metrics.mark_complete("test-1", "accepted");

        let agg = metrics.aggregated();
        assert_eq!(agg.total_pipelines, 1);
        assert_eq!(agg.successful_pipelines, 1);
    }

    #[test]
    fn test_success_rate() {
        let mut metrics = Metrics::new();

        for id in ["test-1", "test-2", "test-3"] {
            metrics.record_phase(PhaseMetrics {
                pipeline_id: id.to_string(),
                phase: "test".to_string(),
                started_at: Utc::now(),
                duration_secs: 1.0,
                success: true,
            });
        }

        metrics.mark_complete("test-1", "accepted");
        metrics.mark_complete("test-2", "accepted");
        metrics.mark_complete("test-3", "failed");

        let rate = metrics.success_rate();
        assert!((rate - 66.666666).abs() < 0.1);
    }

    #[test]
    fn test_slowest_phases() {
        let mut metrics = Metrics::new();

        metrics.record_phase(PhaseMetrics {
            pipeline_id: "test-1".to_string(),
            phase: "fast".to_string(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });

        metrics.record_phase(PhaseMetrics {
            pipeline_id: "test-1".to_string(),
            phase: "slow".to_string(),
            started_at: Utc::now(),
            duration_secs: 10.0,
            success: true,
        });

        let slowest = metrics.slowest_phases(1);
        assert_eq!(slowest[0].0, "slow");
    }
}
