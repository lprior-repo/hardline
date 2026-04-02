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
}
