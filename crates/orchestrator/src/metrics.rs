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

impl std::fmt::Display for ScenarioResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.passed { "PASS" } else { "FAIL" };
        match &self.error {
            Some(err) => write!(
                f,
                "[{}] {} ({:.3}s) — {}",
                status, self.name, self.duration_secs, err
            ),
            None => write!(f, "[{}] {} ({:.3}s)", status, self.name, self.duration_secs),
        }
    }
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

    #[test]
    fn test_scenario_pass_rate_no_results() {
        let metrics = Metrics::new();
        assert_eq!(metrics.scenario_pass_rate(), 0.0);
    }

    #[test]
    fn test_scenario_pass_rate_all_passed() {
        let mut metrics = Metrics::new();

        metrics.record_phase(PhaseMetrics {
            pipeline_id: "test-1".to_string(),
            phase: "validation".to_string(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });

        metrics.record_scenarios(
            "test-1",
            vec![
                ScenarioResult {
                    name: "scenario_1".to_string(),
                    passed: true,
                    duration_secs: 0.5,
                    error: None,
                },
                ScenarioResult {
                    name: "scenario_2".to_string(),
                    passed: true,
                    duration_secs: 0.3,
                    error: None,
                },
            ],
        );

        assert!((metrics.scenario_pass_rate() - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_scenario_pass_rate_partial() {
        let mut metrics = Metrics::new();

        metrics.record_phase(PhaseMetrics {
            pipeline_id: "test-1".to_string(),
            phase: "validation".to_string(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });

        metrics.record_scenarios(
            "test-1",
            vec![
                ScenarioResult {
                    name: "pass".to_string(),
                    passed: true,
                    duration_secs: 0.5,
                    error: None,
                },
                ScenarioResult {
                    name: "fail".to_string(),
                    passed: false,
                    duration_secs: 0.3,
                    error: Some("assertion failed".to_string()),
                },
            ],
        );

        assert!((metrics.scenario_pass_rate() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_get_for_pipeline_filters_correctly() {
        let mut metrics = Metrics::new();

        metrics.record_phase(PhaseMetrics {
            pipeline_id: "alpha".to_string(),
            phase: "spec_review".to_string(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });

        metrics.record_phase(PhaseMetrics {
            pipeline_id: "beta".to_string(),
            phase: "validation".to_string(),
            started_at: Utc::now(),
            duration_secs: 2.0,
            success: false,
        });

        metrics.record_phase(PhaseMetrics {
            pipeline_id: "alpha".to_string(),
            phase: "validation".to_string(),
            started_at: Utc::now(),
            duration_secs: 3.0,
            success: true,
        });

        let alpha_phases = metrics.get_for_pipeline("alpha");
        assert_eq!(alpha_phases.len(), 2);

        let beta_phases = metrics.get_for_pipeline("beta");
        assert_eq!(beta_phases.len(), 1);

        let unknown = metrics.get_for_pipeline("unknown");
        assert!(unknown.is_empty());
    }

    #[test]
    fn test_record_iteration() {
        let mut metrics = Metrics::new();

        metrics.record_phase(PhaseMetrics {
            pipeline_id: "test-1".to_string(),
            phase: "agent_dev".to_string(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });

        metrics.record_iteration("test-1", 5);

        let pipeline = metrics.get_pipeline_metrics("test-1");
        assert!(pipeline.is_some());
        assert_eq!(pipeline.map(|p| p.iteration_count), Some(5));
    }

    #[test]
    fn test_record_iteration_nonexistent_pipeline_is_noop() {
        let mut metrics = Metrics::new();
        metrics.record_iteration("nonexistent", 3);
        assert!(metrics.get_pipeline_metrics("nonexistent").is_none());
    }

    #[test]
    fn test_mark_complete() {
        let mut metrics = Metrics::new();

        metrics.record_phase(PhaseMetrics {
            pipeline_id: "test-1".to_string(),
            phase: "validation".to_string(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });

        metrics.mark_complete("test-1", "accepted");
        assert_eq!(
            metrics
                .get_pipeline_metrics("test-1")
                .map(|p| p.final_state.as_str()),
            Some("accepted")
        );
    }

    #[test]
    fn test_success_rate_empty() {
        let metrics = Metrics::new();
        assert_eq!(metrics.success_rate(), 0.0);
    }

    #[test]
    fn test_aggregated_metrics_counts_failed_and_escalated() {
        let mut metrics = Metrics::new();

        for (id, final_state) in [("p1", "accepted"), ("p2", "failed"), ("p3", "escalated")] {
            metrics.record_phase(PhaseMetrics {
                pipeline_id: id.to_string(),
                phase: "test".to_string(),
                started_at: Utc::now(),
                duration_secs: 1.0,
                success: true,
            });
            metrics.mark_complete(id, final_state);
        }

        let agg = metrics.aggregated();
        assert_eq!(agg.total_pipelines, 3);
        assert_eq!(agg.successful_pipelines, 1);
        assert_eq!(agg.failed_pipelines, 1);
        assert_eq!(agg.escalated_pipelines, 1);
    }

    #[test]
    fn test_aggregated_metrics_empty() {
        let metrics = Metrics::new();
        let agg = metrics.aggregated();
        assert_eq!(agg.total_pipelines, 0);
        assert_eq!(agg.average_duration_secs, 0.0);
    }

    #[test]
    fn test_slowest_phases_empty() {
        let metrics = Metrics::new();
        assert!(metrics.slowest_phases(5).is_empty());
    }

    #[test]
    fn test_slowest_phases_respects_limit() {
        let mut metrics = Metrics::new();

        for i in 0..5 {
            metrics.record_phase(PhaseMetrics {
                pipeline_id: "test-1".to_string(),
                phase: format!("phase_{i}"),
                started_at: Utc::now(),
                duration_secs: f64::from(i + 1),
                success: true,
            });
        }

        let slowest = metrics.slowest_phases(2);
        assert_eq!(slowest.len(), 2);
        assert_eq!(slowest[0].0, "phase_4");
        assert_eq!(slowest[1].0, "phase_3");
    }

    #[test]
    fn test_aggregated_metrics_phase_durations() {
        let mut metrics = Metrics::new();

        metrics.record_phase(PhaseMetrics {
            pipeline_id: "test-1".to_string(),
            phase: "review".to_string(),
            started_at: Utc::now(),
            duration_secs: 2.0,
            success: true,
        });
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "test-1".to_string(),
            phase: "review".to_string(),
            started_at: Utc::now(),
            duration_secs: 4.0,
            success: true,
        });

        let agg = metrics.aggregated();
        assert_eq!(agg.phase_durations.get("review"), Some(&3.0));
    }

    #[test]
    fn test_export_serializes_pipeline_metrics() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "test-1".to_string(),
            phase: "review".to_string(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });

        let exported = metrics.export();
        assert!(exported.is_ok());
        let json = exported.unwrap();
        assert!(json.contains("test-1"));
        assert!(json.contains("review"));
    }

    #[test]
    fn test_get_phase_metrics_iterator() {
        let mut metrics = Metrics::new();
        assert_eq!(metrics.get_phase_metrics().count(), 0);

        metrics.record_phase(PhaseMetrics {
            pipeline_id: "test-1".to_string(),
            phase: "review".to_string(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });

        assert_eq!(metrics.get_phase_metrics().count(), 1);
    }

    #[test]
    fn test_get_pipeline_metrics_nonexistent() {
        let metrics = Metrics::new();
        assert!(metrics.get_pipeline_metrics("nonexistent").is_none());
    }

    // --- Serde roundtrips ---

    #[test]
    fn test_scenario_result_serde_roundtrip() {
        let result = ScenarioResult {
            name: "happy_path".to_string(),
            passed: true,
            duration_secs: 1.5,
            error: None,
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let deserialized: ScenarioResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(result.name, deserialized.name);
        assert_eq!(result.passed, deserialized.passed);
    }

    #[test]
    fn test_scenario_result_with_error_serde_roundtrip() {
        let result = ScenarioResult {
            name: "failure_case".to_string(),
            passed: false,
            duration_secs: 0.3,
            error: Some("assertion failed".to_string()),
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let deserialized: ScenarioResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(result.error, deserialized.error);
    }

    #[test]
    fn test_phase_metrics_serde_roundtrip() {
        let m = PhaseMetrics {
            pipeline_id: "pipe-1".to_string(),
            phase: "spec_review".to_string(),
            started_at: Utc::now(),
            duration_secs: 2.5,
            success: true,
        };
        let json = serde_json::to_string(&m).expect("serialize");
        let deserialized: PhaseMetrics = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(m.pipeline_id, deserialized.pipeline_id);
        assert_eq!(m.phase, deserialized.phase);
        assert_eq!(m.duration_secs, deserialized.duration_secs);
    }

    #[test]
    fn test_aggregated_metrics_serde_roundtrip() {
        let agg = AggregatedMetrics {
            total_pipelines: 5,
            successful_pipelines: 3,
            failed_pipelines: 1,
            escalated_pipelines: 1,
            average_duration_secs: 42.5,
            average_iterations: 2.1,
            phase_durations: {
                let mut map = HashMap::new();
                map.insert("review".to_string(), 3.0);
                map
            },
        };
        let json = serde_json::to_string(&agg).expect("serialize");
        let deserialized: AggregatedMetrics = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(agg.total_pipelines, deserialized.total_pipelines);
        assert_eq!(agg.successful_pipelines, deserialized.successful_pipelines);
        assert_eq!(
            agg.phase_durations.len(),
            deserialized.phase_durations.len()
        );
    }

    #[test]
    fn test_metrics_default_is_empty() {
        let metrics = Metrics::default();
        assert!(metrics.get_phase_metrics().next().is_none());
        assert_eq!(metrics.success_rate(), 0.0);
        assert_eq!(metrics.scenario_pass_rate(), 0.0);
        assert!(metrics.slowest_phases(10).is_empty());
        let agg = metrics.aggregated();
        assert_eq!(agg.total_pipelines, 0);
    }

    // --- Metrics accumulation properties ---

    #[test]
    fn test_multiple_pipelines_metrics_accumulate() {
        let mut metrics = Metrics::new();
        for i in 0..5 {
            metrics.record_phase(PhaseMetrics {
                pipeline_id: format!("pipe-{i}"),
                phase: "test_phase".to_string(),
                started_at: Utc::now(),
                duration_secs: f64::from(i + 1),
                success: true,
            });
            metrics.mark_complete(&format!("pipe-{i}"), "accepted");
        }
        let agg = metrics.aggregated();
        assert_eq!(agg.total_pipelines, 5);
        assert_eq!(agg.successful_pipelines, 5);
        assert_eq!(agg.failed_pipelines, 0);
    }

    #[test]
    fn test_aggregated_average_iterations_with_zero_iterations() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "pipe-1".to_string(),
            phase: "test".to_string(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });
        metrics.mark_complete("pipe-1", "accepted");
        // iteration_count defaults to 0
        let agg = metrics.aggregated();
        assert_eq!(agg.average_iterations, 0.0);
    }

    #[test]
    fn test_record_scenarios_for_nonexistent_pipeline_is_noop() {
        let mut metrics = Metrics::new();
        metrics.record_scenarios(
            "nonexistent",
            vec![ScenarioResult {
                name: "test".to_string(),
                passed: true,
                duration_secs: 1.0,
                error: None,
            }],
        );
        assert_eq!(metrics.scenario_pass_rate(), 0.0);
    }

    #[test]
    fn test_mark_complete_for_nonexistent_pipeline_is_noop() {
        let mut metrics = Metrics::new();
        metrics.mark_complete("nonexistent", "accepted");
        assert!(metrics.get_pipeline_metrics("nonexistent").is_none());
    }

    #[test]
    fn test_get_phase_metrics_returns_all_recorded() {
        let mut metrics = Metrics::new();
        for i in 0..10 {
            metrics.record_phase(PhaseMetrics {
                pipeline_id: "pipe-1".to_string(),
                phase: format!("phase_{i}"),
                started_at: Utc::now(),
                duration_secs: 1.0,
                success: true,
            });
        }
        assert_eq!(metrics.get_phase_metrics().count(), 10);
    }

    #[test]
    fn test_slowest_phases_with_same_durations() {
        let mut metrics = Metrics::new();
        for _ in 0..3 {
            metrics.record_phase(PhaseMetrics {
                pipeline_id: "pipe-1".to_string(),
                phase: "same_phase".to_string(),
                started_at: Utc::now(),
                duration_secs: 5.0,
                success: true,
            });
        }
        let slowest = metrics.slowest_phases(1);
        assert_eq!(slowest.len(), 1);
        // 3 * 5.0 = 15.0
        assert!((slowest[0].1 - 15.0).abs() < 0.01);
    }

    #[test]
    fn test_success_rate_all_failed() {
        let mut metrics = Metrics::new();
        for id in ["p1", "p2"] {
            metrics.record_phase(PhaseMetrics {
                pipeline_id: id.to_string(),
                phase: "test".to_string(),
                started_at: Utc::now(),
                duration_secs: 1.0,
                success: false,
            });
            metrics.mark_complete(id, "failed");
        }
        assert!((metrics.success_rate() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_success_rate_all_accepted() {
        let mut metrics = Metrics::new();
        for id in ["p1", "p2", "p3"] {
            metrics.record_phase(PhaseMetrics {
                pipeline_id: id.to_string(),
                phase: "test".to_string(),
                started_at: Utc::now(),
                duration_secs: 1.0,
                success: true,
            });
            metrics.mark_complete(id, "accepted");
        }
        assert!((metrics.success_rate() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_scenario_pass_rate_all_failed() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "pipe-1".to_string(),
            phase: "val".to_string(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });
        metrics.record_scenarios(
            "pipe-1",
            vec![
                ScenarioResult {
                    name: "s1".to_string(),
                    passed: false,
                    duration_secs: 1.0,
                    error: Some("fail".to_string()),
                },
                ScenarioResult {
                    name: "s2".to_string(),
                    passed: false,
                    duration_secs: 1.0,
                    error: Some("fail".to_string()),
                },
            ],
        );
        assert!((metrics.scenario_pass_rate() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_aggregated_phase_durations_across_pipelines() {
        let mut metrics = Metrics::new();

        // Pipeline 1: phase_a 2.0, phase_a 4.0
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".to_string(),
            phase: "phase_a".to_string(),
            started_at: Utc::now(),
            duration_secs: 2.0,
            success: true,
        });
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".to_string(),
            phase: "phase_a".to_string(),
            started_at: Utc::now(),
            duration_secs: 4.0,
            success: true,
        });

        // Pipeline 2: phase_a 10.0
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p2".to_string(),
            phase: "phase_a".to_string(),
            started_at: Utc::now(),
            duration_secs: 10.0,
            success: true,
        });

        // Average: (2.0 + 4.0 + 10.0) / 3 = 5.33
        let agg = metrics.aggregated();
        let avg = agg.phase_durations.get("phase_a").expect("should exist");
        assert!((*avg - 5.333).abs() < 0.01);
    }

    #[test]
    fn test_record_iteration_updates_count() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".to_string(),
            phase: "dev".to_string(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });
        metrics.record_iteration("p1", 3);
        metrics.record_iteration("p1", 7); // overwrites
        assert_eq!(
            metrics
                .get_pipeline_metrics("p1")
                .map(|p| p.iteration_count),
            Some(7)
        );
    }

    #[test]
    fn test_clear_resets_all_metrics() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".to_string(),
            phase: "test".to_string(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });
        metrics.mark_complete("p1", "accepted");
        assert!(metrics.get_pipeline_metrics("p1").is_some());
        assert_eq!(metrics.get_phase_metrics().count(), 1);

        metrics.clear();
        assert!(metrics.get_pipeline_metrics("p1").is_none());
        assert_eq!(metrics.get_phase_metrics().count(), 0);
    }

    #[test]
    fn test_export_empty_metrics() {
        let metrics = Metrics::new();
        let exported = metrics.export().expect("export");
        assert_eq!(exported.trim(), "{}");
    }

    #[test]
    fn test_multiple_phases_same_pipeline_aggregate_duration() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".to_string(),
            phase: "review".to_string(),
            started_at: Utc::now(),
            duration_secs: 3.0,
            success: true,
        });
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".to_string(),
            phase: "setup".to_string(),
            started_at: Utc::now(),
            duration_secs: 5.0,
            success: true,
        });
        metrics.mark_complete("p1", "accepted");

        let agg = metrics.aggregated();
        // total_duration = 3.0 + 5.0 = 8.0, avg = 8.0 / 1 = 8.0
        assert!((agg.average_duration_secs - 8.0).abs() < 0.01);
    }

    // --- Proptests for metrics accumulation ---

    use proptest::prelude::*;
    use proptest::{prop_assert, prop_assert_eq};

    proptest! {
        #[test]
        fn prop_success_rate_between_0_and_100(
            accepted in 0u32..20u32,
            failed in 0u32..20u32,
        ) {
            let mut metrics = Metrics::new();
            for i in 0..(accepted + failed) {
                let id = format!("pipe-{i}");
                metrics.record_phase(PhaseMetrics {
                    pipeline_id: id.clone(),
                    phase: "test".to_string(),
                    started_at: Utc::now(),
                    duration_secs: 1.0,
                    success: true,
                });
            }
            for i in 0..accepted {
                let id = format!("pipe-{i}");
                metrics.mark_complete(&id, "accepted");
            }
            for i in accepted..(accepted + failed) {
                let id = format!("pipe-{i}");
                metrics.mark_complete(&id, "failed");
            }
            let rate = metrics.success_rate();
            prop_assert!(rate >= 0.0 && rate <= 100.0);
        }

        #[test]
        fn prop_scenario_pass_rate_between_0_and_100(
            passed_count in 0u32..20u32,
            failed_count in 1u32..20u32,
        ) {
            let _total = passed_count + failed_count;
            let mut metrics = Metrics::new();
            metrics.record_phase(PhaseMetrics {
                pipeline_id: "p1".to_string(),
                phase: "val".to_string(),
                started_at: Utc::now(),
                duration_secs: 1.0,
                success: true,
            });
            let mut results = Vec::new();
            for i in 0..passed_count {
                results.push(ScenarioResult {
                    name: format!("pass-{i}"),
                    passed: true,
                    duration_secs: 1.0,
                    error: None,
                });
            }
            for i in 0..failed_count {
                results.push(ScenarioResult {
                    name: format!("fail-{i}"),
                    passed: false,
                    duration_secs: 1.0,
                    error: Some("err".to_string()),
                });
            }
            metrics.record_scenarios("p1", results);
            let rate = metrics.scenario_pass_rate();
            prop_assert!(rate >= 0.0 && rate <= 100.0);
        }

        #[test]
        fn prop_aggregated_total_equals_recorded_count(count in 1u32..50u32) {
            let mut metrics = Metrics::new();
            for i in 0..count {
                metrics.record_phase(PhaseMetrics {
                    pipeline_id: format!("pipe-{i}"),
                    phase: "test".to_string(),
                    started_at: Utc::now(),
                    duration_secs: 1.0,
                    success: true,
                });
            }
            let agg = metrics.aggregated();
            prop_assert_eq!(agg.total_pipelines, count);
        }
    }

    // ========================================================================
    // ha-ga0: PhaseMetrics — timing, success/failure counts, concurrent recording
    // ========================================================================

    // --- Timing measurement (start/stop/duration) ---

    #[test]
    fn test_phase_metrics_started_at_preserved() {
        let before = Utc::now();
        let pm = PhaseMetrics {
            pipeline_id: "pipe-1".into(),
            phase: "spec_review".into(),
            started_at: before,
            duration_secs: 2.5,
            success: true,
        };
        let after = Utc::now();
        assert!(pm.started_at >= before);
        assert!(pm.started_at <= after);
    }

    #[test]
    fn test_phase_metrics_duration_reflects_elapsed_time() {
        let start = Utc::now();
        let duration = 3.7;
        let pm = PhaseMetrics {
            pipeline_id: "pipe-1".into(),
            phase: "dev".into(),
            started_at: start,
            duration_secs: duration,
            success: true,
        };
        assert!((pm.duration_secs - duration).abs() < f64::EPSILON);
    }

    #[test]
    fn test_phase_metrics_zero_duration_instant_phase() {
        let now = Utc::now();
        let pm = PhaseMetrics {
            pipeline_id: "pipe-1".into(),
            phase: "noop".into(),
            started_at: now,
            duration_secs: 0.0,
            success: true,
        };
        assert!((pm.duration_secs).abs() < f64::EPSILON);
    }

    #[test]
    fn test_phase_metrics_duration_accumulates_in_pipeline() {
        let mut metrics = Metrics::new();
        let now = Utc::now();

        metrics.record_phase(PhaseMetrics {
            pipeline_id: "pipe-1".into(),
            phase: "setup".into(),
            started_at: now,
            duration_secs: 1.0,
            success: true,
        });
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "pipe-1".into(),
            phase: "review".into(),
            started_at: now,
            duration_secs: 2.0,
            success: true,
        });
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "pipe-1".into(),
            phase: "validation".into(),
            started_at: now,
            duration_secs: 3.5,
            success: true,
        });

        let pipeline = metrics.get_pipeline_metrics("pipe-1").expect("exists");
        // total_duration_secs accumulates across phases
        assert!((pipeline.total_duration_secs - 6.5).abs() < 0.01);
        // Each phase retains its own duration
        assert_eq!(pipeline.phase_metrics.len(), 3);
        assert!((pipeline.phase_metrics[0].duration_secs - 1.0).abs() < f64::EPSILON);
        assert!((pipeline.phase_metrics[1].duration_secs - 2.0).abs() < f64::EPSILON);
        assert!((pipeline.phase_metrics[2].duration_secs - 3.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_phase_metrics_started_at_ordering_preserved() {
        let t1 = Utc::now();
        let t2 = t1 + chrono::Duration::seconds(5);
        let t3 = t1 + chrono::Duration::seconds(10);

        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "pipe-1".into(),
            phase: "first".into(),
            started_at: t1,
            duration_secs: 5.0,
            success: true,
        });
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "pipe-1".into(),
            phase: "second".into(),
            started_at: t2,
            duration_secs: 5.0,
            success: true,
        });
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "pipe-1".into(),
            phase: "third".into(),
            started_at: t3,
            duration_secs: 5.0,
            success: true,
        });

        let phases: Vec<_> = metrics.get_for_pipeline("pipe-1");
        assert_eq!(phases.len(), 3);
        assert!(phases[0].started_at < phases[1].started_at);
        assert!(phases[1].started_at < phases[2].started_at);
    }

    // --- Success and failure count tracking ---

    #[test]
    fn test_phase_metrics_counts_successes_and_failures() {
        let mut metrics = Metrics::new();
        let now = Utc::now();

        // Record 3 successes
        for i in 0..3u32 {
            metrics.record_phase(PhaseMetrics {
                pipeline_id: format!("pipe-{i}"),
                phase: "test".into(),
                started_at: now,
                duration_secs: 1.0,
                success: true,
            });
        }
        // Record 2 failures
        for i in 3..5u32 {
            metrics.record_phase(PhaseMetrics {
                pipeline_id: format!("pipe-{i}"),
                phase: "test".into(),
                started_at: now,
                duration_secs: 1.0,
                success: false,
            });
        }

        let all_phases: Vec<_> = metrics.get_phase_metrics().collect();
        let successes = all_phases.iter().filter(|m| m.success).count();
        let failures = all_phases.iter().filter(|m| !m.success).count();

        assert_eq!(successes, 3);
        assert_eq!(failures, 2);
    }

    #[test]
    fn test_phase_metrics_success_field_in_individual_records() {
        let now = Utc::now();
        let success_pm = PhaseMetrics {
            pipeline_id: "pipe-1".into(),
            phase: "review".into(),
            started_at: now,
            duration_secs: 1.0,
            success: true,
        };
        let failure_pm = PhaseMetrics {
            pipeline_id: "pipe-2".into(),
            phase: "review".into(),
            started_at: now,
            duration_secs: 1.0,
            success: false,
        };
        assert!(success_pm.success);
        assert!(!failure_pm.success);
    }

    #[test]
    fn test_phase_metrics_pipeline_success_rate_from_final_state() {
        let mut metrics = Metrics::new();
        let now = Utc::now();

        // 4 pipelines, 3 accepted, 1 failed
        for i in 0..4u32 {
            metrics.record_phase(PhaseMetrics {
                pipeline_id: format!("p{i}"),
                phase: "test".into(),
                started_at: now,
                duration_secs: 1.0,
                success: true,
            });
        }
        metrics.mark_complete("p0", "accepted");
        metrics.mark_complete("p1", "accepted");
        metrics.mark_complete("p2", "accepted");
        metrics.mark_complete("p3", "failed");

        assert!((metrics.success_rate() - 75.0).abs() < 0.01);
    }

    #[test]
    fn test_phase_metrics_all_success_pure() {
        let mut metrics = Metrics::new();
        let now = Utc::now();
        for i in 0..5u32 {
            metrics.record_phase(PhaseMetrics {
                pipeline_id: format!("p{i}"),
                phase: "test".into(),
                started_at: now,
                duration_secs: 1.0,
                success: true,
            });
            metrics.mark_complete(&format!("p{i}"), "accepted");
        }
        assert!((metrics.success_rate() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_phase_metrics_all_failure() {
        let mut metrics = Metrics::new();
        let now = Utc::now();
        for i in 0..3u32 {
            metrics.record_phase(PhaseMetrics {
                pipeline_id: format!("p{i}"),
                phase: "test".into(),
                started_at: now,
                duration_secs: 1.0,
                success: false,
            });
            metrics.mark_complete(&format!("p{i}"), "failed");
        }
        assert!((metrics.success_rate() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_phase_metrics_mixed_success_within_single_pipeline() {
        let mut metrics = Metrics::new();
        let now = Utc::now();

        // Single pipeline with 2 successful phases and 1 failed phase
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "pipe-1".into(),
            phase: "setup".into(),
            started_at: now,
            duration_secs: 1.0,
            success: true,
        });
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "pipe-1".into(),
            phase: "review".into(),
            started_at: now,
            duration_secs: 2.0,
            success: true,
        });
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "pipe-1".into(),
            phase: "validation".into(),
            started_at: now,
            duration_secs: 3.0,
            success: false,
        });

        let phases = metrics.get_for_pipeline("pipe-1");
        let successes = phases.iter().filter(|m| m.success).count();
        let failures = phases.iter().filter(|m| !m.success).count();
        assert_eq!(successes, 2);
        assert_eq!(failures, 1);
    }

    // --- Concurrent phase metric recording ---

    #[test]
    fn test_concurrent_recording_from_multiple_phases() {
        // Simulate interleaved recording of multiple phases as would happen
        // with concurrent pipeline execution
        let mut metrics = Metrics::new();
        let now = Utc::now();

        // Interleave phase recordings from 3 different pipelines
        for i in 0..10 {
            for pipe in ["alpha", "beta", "gamma"] {
                metrics.record_phase(PhaseMetrics {
                    pipeline_id: pipe.to_string(),
                    phase: format!("step_{i}"),
                    started_at: now,
                    duration_secs: 1.0,
                    success: true,
                });
            }
        }

        // Each pipeline should have exactly 10 phases
        assert_eq!(metrics.get_for_pipeline("alpha").len(), 10);
        assert_eq!(metrics.get_for_pipeline("beta").len(), 10);
        assert_eq!(metrics.get_for_pipeline("gamma").len(), 10);

        // Total phase metrics should be 30
        assert_eq!(metrics.get_phase_metrics().count(), 30);

        // Each pipeline accumulated duration
        let alpha = metrics.get_pipeline_metrics("alpha").expect("exists");
        assert!((alpha.total_duration_secs - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_concurrent_recording_does_not_cross_contaminate() {
        let mut metrics = Metrics::new();
        let now = Utc::now();

        // Pipeline A: 3 phases, all succeed
        for phase in ["a1", "a2", "a3"] {
            metrics.record_phase(PhaseMetrics {
                pipeline_id: "pipe-a".into(),
                phase: phase.into(),
                started_at: now,
                duration_secs: 1.0,
                success: true,
            });
        }

        // Pipeline B: 2 phases, one fails
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "pipe-b".into(),
            phase: "b1".into(),
            started_at: now,
            duration_secs: 5.0,
            success: true,
        });
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "pipe-b".into(),
            phase: "b2".into(),
            started_at: now,
            duration_secs: 5.0,
            success: false,
        });

        // Verify isolation
        let a_phases = metrics.get_for_pipeline("pipe-a");
        let b_phases = metrics.get_for_pipeline("pipe-b");

        assert_eq!(a_phases.len(), 3);
        assert!(a_phases.iter().all(|m| m.success));

        assert_eq!(b_phases.len(), 2);
        assert_eq!(b_phases.iter().filter(|m| m.success).count(), 1);
        assert_eq!(b_phases.iter().filter(|m| !m.success).count(), 1);

        let a_pipeline = metrics.get_pipeline_metrics("pipe-a").expect("exists");
        assert!((a_pipeline.total_duration_secs - 3.0).abs() < 0.01);

        let b_pipeline = metrics.get_pipeline_metrics("pipe-b").expect("exists");
        assert!((b_pipeline.total_duration_secs - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_concurrent_recording_aggregation_correctness() {
        let mut metrics = Metrics::new();
        let now = Utc::now();

        // Record phases for 5 pipelines simultaneously
        for i in 0..5u32 {
            metrics.record_phase(PhaseMetrics {
                pipeline_id: format!("pipe-{i}"),
                phase: "setup".into(),
                started_at: now,
                duration_secs: f64::from(i + 1),
                success: true,
            });
            metrics.record_phase(PhaseMetrics {
                pipeline_id: format!("pipe-{i}"),
                phase: "validation".into(),
                started_at: now,
                duration_secs: f64::from(i + 1) * 2.0,
                success: i % 2 == 0,
            });
            metrics.mark_complete(
                &format!("pipe-{i}"),
                if i % 2 == 0 { "accepted" } else { "failed" },
            );
        }

        let agg = metrics.aggregated();
        assert_eq!(agg.total_pipelines, 5);
        // pipes 0, 2, 4 accepted → 3 successful
        assert_eq!(agg.successful_pipelines, 3);
        // pipes 1, 3 failed → 2 failed
        assert_eq!(agg.failed_pipelines, 2);
    }

    #[test]
    fn test_phase_metrics_serde_roundtrip_with_timing() {
        let t = "2026-04-05T12:34:56.789Z".parse::<DateTime<Utc>>().unwrap();
        let pm = PhaseMetrics {
            pipeline_id: "pipe-42".into(),
            phase: "validation".into(),
            started_at: t,
            duration_secs: 7.891,
            success: false,
        };
        let json = serde_json::to_string(&pm).expect("serialize");
        let back: PhaseMetrics = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.pipeline_id, "pipe-42");
        assert_eq!(back.phase, "validation");
        assert_eq!(back.started_at, t);
        assert!((back.duration_secs - 7.891).abs() < 1e-6);
        assert!(!back.success);
    }

    #[test]
    fn test_phase_metrics_json_preserves_all_fields() {
        let t = Utc::now();
        let pm = PhaseMetrics {
            pipeline_id: "p1".into(),
            phase: "review".into(),
            started_at: t,
            duration_secs: 4.2,
            success: true,
        };
        let val: serde_json::Value = serde_json::to_value(&pm).expect("to_value");
        assert_eq!(val["pipeline_id"], "p1");
        assert_eq!(val["phase"], "review");
        assert_eq!(val["duration_secs"], 4.2);
        assert_eq!(val["success"], true);
        assert!(val["started_at"].is_string());
    }

    // ========================================================================
    // ha-ah8: Exhaustive ScenarioResult tests — construction, serde, display,
    //         aggregation from multiple phases
    // ========================================================================

    // --- Construction ---

    #[test]
    fn test_scenario_result_construct_pass() {
        let r = ScenarioResult {
            name: "login_flow".into(),
            passed: true,
            duration_secs: 1.234,
            error: None,
        };
        assert_eq!(r.name, "login_flow");
        assert!(r.passed);
        assert!((r.duration_secs - 1.234).abs() < f64::EPSILON);
        assert!(r.error.is_none());
    }

    #[test]
    fn test_scenario_result_construct_fail_with_error() {
        let r = ScenarioResult {
            name: "checkout".into(),
            passed: false,
            duration_secs: 0.5,
            error: Some("assertion failed: status 404".into()),
        };
        assert!(!r.passed);
        assert_eq!(r.error.as_deref(), Some("assertion failed: status 404"));
    }

    #[test]
    fn test_scenario_result_construct_fail_no_error_message() {
        // A result can fail without an error message
        let r = ScenarioResult {
            name: "timeout_test".into(),
            passed: false,
            duration_secs: 30.0,
            error: None,
        };
        assert!(!r.passed);
        assert!(r.error.is_none());
    }

    #[test]
    fn test_scenario_result_zero_duration() {
        let r = ScenarioResult {
            name: "instant".into(),
            passed: true,
            duration_secs: 0.0,
            error: None,
        };
        assert!((r.duration_secs).abs() < f64::EPSILON);
    }

    #[test]
    fn test_scenario_result_empty_name() {
        let r = ScenarioResult {
            name: String::new(),
            passed: true,
            duration_secs: 1.0,
            error: None,
        };
        assert!(r.name.is_empty());
    }

    #[test]
    fn test_scenario_result_clone_preserves_fields() {
        let original = ScenarioResult {
            name: "deep_copy_test".into(),
            passed: true,
            duration_secs: 2.5,
            error: Some("minor warning".into()),
        };
        let cloned = original.clone();
        assert_eq!(original.name, cloned.name);
        assert_eq!(original.passed, cloned.passed);
        assert_eq!(original.duration_secs, cloned.duration_secs);
        assert_eq!(original.error, cloned.error);
    }

    // --- Serialization round-trip (JSON) ---

    #[test]
    fn test_scenario_result_serde_pass_roundtrip() {
        let r = ScenarioResult {
            name: "happy_path".into(),
            passed: true,
            duration_secs: 1.5,
            error: None,
        };
        let json = serde_json::to_string(&r).expect("serialize");
        let back: ScenarioResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r.name, back.name);
        assert_eq!(r.passed, back.passed);
        assert!((r.duration_secs - back.duration_secs).abs() < f64::EPSILON);
        assert_eq!(r.error, back.error);
    }

    #[test]
    fn test_scenario_result_serde_fail_roundtrip() {
        let r = ScenarioResult {
            name: "failure_case".into(),
            passed: false,
            duration_secs: 0.3,
            error: Some("assertion failed".into()),
        };
        let json = serde_json::to_string(&r).expect("serialize");
        let back: ScenarioResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r.error, back.error);
        assert!(!back.passed);
    }

    #[test]
    fn test_scenario_result_json_structure() {
        let r = ScenarioResult {
            name: "struct_test".into(),
            passed: true,
            duration_secs: 3.14,
            error: None,
        };
        let val: serde_json::Value = serde_json::to_value(&r).expect("to_value");
        assert_eq!(val["name"], "struct_test");
        assert_eq!(val["passed"], true);
        assert_eq!(val["duration_secs"], 3.14);
        assert!(val["error"].is_null());
    }

    #[test]
    fn test_scenario_result_json_structure_with_error() {
        let r = ScenarioResult {
            name: "err_struct".into(),
            passed: false,
            duration_secs: 0.1,
            error: Some("boom".into()),
        };
        let val: serde_json::Value = serde_json::to_value(&r).expect("to_value");
        assert_eq!(val["error"], "boom");
    }

    #[test]
    fn test_scenario_result_pretty_json_roundtrip() {
        let r = ScenarioResult {
            name: "pretty".into(),
            passed: true,
            duration_secs: 99.9,
            error: Some("warning".into()),
        };
        let json = serde_json::to_string_pretty(&r).expect("pretty serialize");
        let back: ScenarioResult = serde_json::from_str(&json).expect("pretty deserialize");
        assert_eq!(r.name, back.name);
        assert_eq!(r.error, back.error);
    }

    #[test]
    fn test_scenario_result_deserialize_from_literal_json() {
        let json = r#"{"name":"literal","passed":true,"duration_secs":5.0,"error":null}"#;
        let r: ScenarioResult = serde_json::from_str(json).expect("deserialize literal");
        assert_eq!(r.name, "literal");
        assert!(r.passed);
        assert!(r.error.is_none());
    }

    #[test]
    fn test_scenario_result_vec_serde_roundtrip() {
        let results = vec![
            ScenarioResult {
                name: "a".into(),
                passed: true,
                duration_secs: 1.0,
                error: None,
            },
            ScenarioResult {
                name: "b".into(),
                passed: false,
                duration_secs: 2.0,
                error: Some("fail".into()),
            },
        ];
        let json = serde_json::to_string(&results).expect("serialize vec");
        let back: Vec<ScenarioResult> = serde_json::from_str(&json).expect("deserialize vec");
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].name, "a");
        assert!(back[0].passed);
        assert!(!back[1].passed);
        assert_eq!(back[1].error.as_deref(), Some("fail"));
    }

    // --- Display formatting ---

    #[test]
    fn test_scenario_result_display_pass_no_error() {
        let r = ScenarioResult {
            name: "login_test".into(),
            passed: true,
            duration_secs: 1.5,
            error: None,
        };
        let display = format!("{r}");
        assert!(display.contains("[PASS]"));
        assert!(display.contains("login_test"));
        assert!(display.contains("1.500"));
        assert!(!display.contains("—"));
    }

    #[test]
    fn test_scenario_result_display_fail_with_error() {
        let r = ScenarioResult {
            name: "checkout".into(),
            passed: false,
            duration_secs: 0.25,
            error: Some("status 500".into()),
        };
        let display = format!("{r}");
        assert!(display.contains("[FAIL]"));
        assert!(display.contains("checkout"));
        assert!(display.contains("0.250"));
        assert!(display.contains("—"));
        assert!(display.contains("status 500"));
    }

    #[test]
    fn test_scenario_result_display_fail_no_error() {
        // FAIL without error message — no em-dash
        let r = ScenarioResult {
            name: "timeout".into(),
            passed: false,
            duration_secs: 30.0,
            error: None,
        };
        let display = format!("{r}");
        assert!(display.contains("[FAIL]"));
        assert!(!display.contains("—"));
    }

    #[test]
    fn test_scenario_result_display_zero_duration() {
        let r = ScenarioResult {
            name: "instant".into(),
            passed: true,
            duration_secs: 0.0,
            error: None,
        };
        let display = format!("{r}");
        assert!(display.contains("0.000"));
    }

    #[test]
    fn test_scenario_result_display_empty_name() {
        let r = ScenarioResult {
            name: String::new(),
            passed: true,
            duration_secs: 1.0,
            error: None,
        };
        let display = format!("{r}");
        assert!(display.contains("[PASS]"));
    }

    #[test]
    fn test_scenario_result_display_multiline_error() {
        let r = ScenarioResult {
            name: "multi".into(),
            passed: false,
            duration_secs: 1.0,
            error: Some("line1\nline2\nline3".into()),
        };
        let display = format!("{r}");
        assert!(display.contains("line1\nline2\nline3"));
    }

    // --- Aggregation from multiple phases ---

    #[test]
    fn test_scenario_aggregation_all_pass_100_percent() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".into(),
            phase: "validation".into(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });
        metrics.record_scenarios(
            "p1",
            vec![
                ScenarioResult {
                    name: "s1".into(),
                    passed: true,
                    duration_secs: 0.5,
                    error: None,
                },
                ScenarioResult {
                    name: "s2".into(),
                    passed: true,
                    duration_secs: 0.3,
                    error: None,
                },
                ScenarioResult {
                    name: "s3".into(),
                    passed: true,
                    duration_secs: 0.2,
                    error: None,
                },
            ],
        );
        assert!((metrics.scenario_pass_rate() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_scenario_aggregation_mixed_pass_fail() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".into(),
            phase: "validation".into(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });
        metrics.record_scenarios(
            "p1",
            vec![
                ScenarioResult {
                    name: "pass".into(),
                    passed: true,
                    duration_secs: 1.0,
                    error: None,
                },
                ScenarioResult {
                    name: "fail1".into(),
                    passed: false,
                    duration_secs: 2.0,
                    error: Some("err".into()),
                },
                ScenarioResult {
                    name: "fail2".into(),
                    passed: false,
                    duration_secs: 3.0,
                    error: Some("err".into()),
                },
            ],
        );
        assert!((metrics.scenario_pass_rate() - 33.333).abs() < 0.1);
    }

    #[test]
    fn test_scenario_aggregation_across_multiple_pipelines() {
        let mut metrics = Metrics::new();

        // Pipeline 1: 2 passed, 1 failed
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".into(),
            phase: "validation".into(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });
        metrics.record_scenarios(
            "p1",
            vec![
                ScenarioResult {
                    name: "p1_s1".into(),
                    passed: true,
                    duration_secs: 0.1,
                    error: None,
                },
                ScenarioResult {
                    name: "p1_s2".into(),
                    passed: true,
                    duration_secs: 0.2,
                    error: None,
                },
                ScenarioResult {
                    name: "p1_s3".into(),
                    passed: false,
                    duration_secs: 0.3,
                    error: Some("fail".into()),
                },
            ],
        );

        // Pipeline 2: 1 passed
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p2".into(),
            phase: "validation".into(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });
        metrics.record_scenarios(
            "p2",
            vec![ScenarioResult {
                name: "p2_s1".into(),
                passed: true,
                duration_secs: 0.5,
                error: None,
            }],
        );

        // Total: 3 passed, 1 failed out of 4 = 75%
        assert!((metrics.scenario_pass_rate() - 75.0).abs() < 0.01);
    }

    #[test]
    fn test_scenario_aggregation_overwrite_via_record_scenarios() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".into(),
            phase: "val".into(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });

        // First batch: all pass
        metrics.record_scenarios(
            "p1",
            vec![ScenarioResult {
                name: "s1".into(),
                passed: true,
                duration_secs: 1.0,
                error: None,
            }],
        );
        assert!((metrics.scenario_pass_rate() - 100.0).abs() < 0.01);

        // Overwrite: all fail
        metrics.record_scenarios(
            "p1",
            vec![ScenarioResult {
                name: "s1".into(),
                passed: false,
                duration_secs: 1.0,
                error: Some("replaced".into()),
            }],
        );
        assert!((metrics.scenario_pass_rate() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_scenario_aggregation_preserved_through_export() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".into(),
            phase: "val".into(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });
        let results = vec![
            ScenarioResult {
                name: "s1".into(),
                passed: true,
                duration_secs: 0.5,
                error: None,
            },
            ScenarioResult {
                name: "s2".into(),
                passed: false,
                duration_secs: 0.5,
                error: Some("err".into()),
            },
        ];
        metrics.record_scenarios("p1", results.clone());

        let exported = metrics.export().expect("export");
        let parsed: std::collections::HashMap<String, PipelineMetrics> =
            serde_json::from_str(&exported).expect("parse export");
        let pm = parsed.get("p1").expect("pipeline p1");
        assert_eq!(pm.scenario_results.len(), 2);
        assert_eq!(pm.scenario_results[0].name, "s1");
        assert!(pm.scenario_results[0].passed);
        assert!(!pm.scenario_results[1].passed);
    }

    // --- Proptests for ScenarioResult serde ---

    proptest! {
        #[test]
        fn prop_scenario_result_serde_roundtrip(
            name in "[a-zA-Z0-9_ ]{0,20}",
            passed: bool,
            duration_secs in 0.0f64..1e6,
            error in proptest::option::of("[a-zA-Z0-9_ ]{0,30}"),
        ) {
            let r = ScenarioResult {
                name: name.clone(),
                passed,
                duration_secs,
                error: error.clone(),
            };
            let json = serde_json::to_string(&r).expect("serialize");
            let back: ScenarioResult = serde_json::from_str(&json).expect("deserialize");
            prop_assert_eq!(back.name, name);
            prop_assert_eq!(back.passed, passed);
            prop_assert_eq!(back.error, error);
            prop_assert!((back.duration_secs - duration_secs).abs() < 1e-6);
        }

        #[test]
        fn prop_scenario_result_display_contains_status(
            passed: bool,
            error in proptest::option::of("error_.*"),
        ) {
            let r = ScenarioResult {
                name: "test".into(),
                passed,
                duration_secs: 1.0,
                error,
            };
            let display = format!("{r}");
            if passed {
                prop_assert!(display.contains("[PASS]"));
            } else {
                prop_assert!(display.contains("[FAIL]"));
            }
        }

        #[test]
        fn prop_scenario_pass_rate_with_n_results(
            n_passed in 0u32..20u32,
            n_failed in 0u32..20u32,
        ) {
            let mut metrics = Metrics::new();
            metrics.record_phase(PhaseMetrics {
                pipeline_id: "p".into(),
                phase: "v".into(),
                started_at: Utc::now(),
                duration_secs: 1.0,
                success: true,
            });

            let total = n_passed + n_failed;
            let mut results = Vec::new();
            for i in 0..n_passed {
                results.push(ScenarioResult {
                    name: format!("p{i}"),
                    passed: true,
                    duration_secs: 1.0,
                    error: None,
                });
            }
            for i in 0..n_failed {
                results.push(ScenarioResult {
                    name: format!("f{i}"),
                    passed: false,
                    duration_secs: 1.0,
                    error: Some("err".into()),
                });
            }
            metrics.record_scenarios("p", results);

            let rate = metrics.scenario_pass_rate();
            if total == 0 {
                prop_assert_eq!(rate, 0.0);
            } else {
                let expected = (n_passed as f64 / total as f64) * 100.0;
                prop_assert!((rate - expected).abs() < 0.01);
            }
        }
    }
}
