//! Metrics collection

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

use super::logging::KeyValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: MetricValue,
    pub timestamp: DateTime<Utc>,
    pub attributes: Vec<KeyValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum MetricValue {
    Counter { value: f64 },
    Gauge { value: f64 },
    Histogram { values: Vec<f64>, count: usize },
}

#[derive(Debug, Clone)]
pub struct Histogram {
    values: Vec<f64>,
    sum: f64,
    count: usize,
}

impl Histogram {
    #[must_use]
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            sum: 0.0,
            count: 0,
        }
    }

    pub fn record(&mut self, value: f64) {
        self.values.push(value);
        self.sum += value;
        self.count += 1;
    }

    #[must_use]
    pub fn get_values(&self) -> &[f64] {
        &self.values
    }

    #[must_use]
    pub fn get_sum(&self) -> f64 {
        self.sum
    }

    #[must_use]
    pub fn get_count(&self) -> usize {
        self.count
    }
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MetricsCollector {
    counters: RwLock<HashMap<String, f64>>,
    gauges: RwLock<HashMap<String, f64>>,
    histograms: RwLock<HashMap<String, Histogram>>,
}

impl MetricsCollector {
    #[must_use]
    pub fn new() -> Self {
        Self {
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
            histograms: RwLock::new(HashMap::new()),
        }
    }

    pub fn increment_counter(&self, name: &str, value: f64, _attributes: Vec<KeyValue>) {
        let mut counters = match self.counters.write() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        *counters.entry(name.to_string()).or_insert(0.0) += value;
    }

    pub fn set_gauge(&self, name: &str, value: f64, _attributes: Vec<KeyValue>) {
        let mut gauges = match self.gauges.write() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        gauges.insert(name.to_string(), value);
    }

    pub fn record_histogram(&self, name: &str, value: f64, _attributes: Vec<KeyValue>) {
        let mut histograms = match self.histograms.write() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        let hist = histograms
            .entry(name.to_string())
            .or_insert_with(Histogram::new);
        hist.record(value);
    }

    #[must_use]
    pub fn export(&self) -> Vec<Metric> {
        let mut metrics = Vec::new();
        let now = Utc::now();

        let counters = match self.counters.read() {
            Ok(guard) => guard,
            Err(_) => return metrics,
        };
        for (name, value) in counters.iter() {
            metrics.push(Metric {
                name: name.clone(),
                value: MetricValue::Counter { value: *value },
                timestamp: now,
                attributes: Vec::new(),
            });
        }

        let gauges = match self.gauges.read() {
            Ok(guard) => guard,
            Err(_) => return metrics,
        };
        for (name, value) in gauges.iter() {
            metrics.push(Metric {
                name: name.clone(),
                value: MetricValue::Gauge { value: *value },
                timestamp: now,
                attributes: Vec::new(),
            });
        }

        let histograms = match self.histograms.read() {
            Ok(guard) => guard,
            Err(_) => return metrics,
        };
        for (name, hist) in histograms.iter() {
            metrics.push(Metric {
                name: name.clone(),
                value: MetricValue::Histogram {
                    values: hist.get_values().to_vec(),
                    count: hist.get_count(),
                },
                timestamp: now,
                attributes: vec![KeyValue {
                    key: "sum".to_string(),
                    value: serde_json::json!(hist.get_sum()),
                }],
            });
        }

        metrics
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_histogram_record() {
        let mut hist = Histogram::new();
        hist.record(1.0);
        hist.record(2.0);
        hist.record(3.0);

        assert_eq!(hist.get_values(), &[1.0, 2.0, 3.0]);
        assert_eq!(hist.get_sum(), 6.0);
        assert_eq!(hist.get_count(), 3);
    }

    #[test]
    fn test_metrics_collector_counter() {
        let collector = MetricsCollector::new();
        collector.increment_counter("test_counter", 5.0, vec![]);
        collector.increment_counter("test_counter", 3.0, vec![]);

        let metrics = collector.export();
        let counter_metric = metrics
            .iter()
            .find(|m| m.name == "test_counter")
            .expect("counter should exist");

        if let MetricValue::Counter { value } = counter_metric.value {
            assert_eq!(value, 8.0);
        } else {
            panic!("expected counter metric");
        }
    }

    #[test]
    fn test_metrics_collector_gauge() {
        let collector = MetricsCollector::new();
        collector.set_gauge("test_gauge", 10.0, vec![]);
        collector.set_gauge("test_gauge", 20.0, vec![]);

        let metrics = collector.export();
        let gauge_metric = metrics
            .iter()
            .find(|m| m.name == "test_gauge")
            .expect("gauge should exist");

        if let MetricValue::Gauge { value } = gauge_metric.value {
            assert_eq!(value, 20.0);
        } else {
            panic!("expected gauge metric");
        }
    }

    #[test]
    fn test_metrics_collector_histogram() {
        let collector = MetricsCollector::new();
        collector.record_histogram("test_histogram", 1.0, vec![]);
        collector.record_histogram("test_histogram", 2.0, vec![]);

        let metrics = collector.export();
        let hist_metric = metrics
            .iter()
            .find(|m| m.name == "test_histogram")
            .expect("histogram should exist");

        if let MetricValue::Histogram { values, count } = hist_metric.value {
            assert_eq!(count, 2);
            assert_eq!(values, &[1.0, 2.0]);
        } else {
            panic!("expected histogram metric");
        }
    }

    #[test]
    fn test_metric_serialization() {
        let metric = Metric {
            name: "test".to_string(),
            value: MetricValue::Counter { value: 42.0 },
            timestamp: Utc::now(),
            attributes: vec![],
        };

        let json = serde_json::to_string(&metric).expect("should serialize");
        assert!(json.contains("\"type\":\"Counter\""));
        assert!(json.contains("42.0"));
    }
}
