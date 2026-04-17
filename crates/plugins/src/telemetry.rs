//! OpenTelemetry Integration for OpenClaw
//!
//! Provides metrics, tracing, and logging integration for observability.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Metrics collector for OpenClaw runtime
#[derive(Debug)]
pub struct MetricsCollector {
    counters: Arc<RwLock<HashMap<String, Counter>>>,
    gauges: Arc<RwLock<HashMap<String, f64>>>,
    histograms: Arc<RwLock<HashMap<String, Histogram>>>,
    start_time: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Counter {
    pub name: String,
    pub value: u64,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Histogram {
    pub name: String,
    #[serde(default)]
    pub values: Vec<f64>,
    #[serde(default)]
    pub buckets: HashMap<usize, usize>, // bucket -> count
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self { counters: Arc::new(RwLock::new(HashMap::new())), gauges: Arc::new(RwLock::new(HashMap::new())), histograms: Arc::new(RwLock::new(HashMap::new())), start_time: Instant::now() }
    }

    /// Increment a counter
    pub fn increment_counter(&self, name: &str, labels: Option<HashMap<String, String>>) {
        let mut counters = self.counters.write().unwrap();
        let key = Self::make_key(name, &labels);
        counters
            .entry(key)
            .or_insert(Counter {
                name: name.into(),
                value: 0,
                labels: labels.unwrap_or_default(),
            })
            .value += 1;
    }

    /// Set a gauge value
    pub fn set_gauge(&self, name: &str, value: f64, labels: Option<HashMap<String, String>>) {
        let mut gauges = self.gauges.write().unwrap();
        let key = Self::make_key(name, &labels);
        gauges.insert(key, value);
    }

    /// Observe a histogram value
    pub fn observe_histogram(&self, name: &str, value: f64, labels: Option<HashMap<String, String>>) {
        let mut histograms = self.histograms.write().unwrap();
        let key = Self::make_key(name, &labels);
        let hist = histograms.entry(key).or_insert(Histogram {
            name: name.into(),
            values: Vec::new(),
            buckets: HashMap::new(),
        });
        hist.values.push(value);
        
        // Update buckets (for percentile calculation)
        let bucket = (value as usize) / 10; // 10-unit buckets
        *hist.buckets.entry(bucket).or_insert(0) += 1;
    }

    /// Record a duration
    pub fn record_duration(&self, name: &str, duration: Duration) {
        self.observe_histogram(&format!("{}_seconds", name), duration.as_secs_f64(), None);
    }

    /// Get all metrics as JSON for export
    pub fn export_json(&self) -> MetricsExport {
        let counters = self.counters.read().unwrap();
        let gauges = self.gauges.read().unwrap();
        let histograms = self.histograms.read().unwrap();
        let uptime = self.start_time.elapsed().as_secs_f64();

        MetricsExport {
            timestamp: chrono::Utc::now().to_rfc3339(),
            uptime_seconds: uptime,
            counters: counters.clone(),
            gauges: gauges.clone(),
            histograms: histograms.clone(),
        }
    }

    /// Export in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let export = self.export_json();
        let mut output = String::new();

        // Counters
        for counter in export.counters.values() {
            let labels = Self::format_labels(&counter.labels);
            output.push_str(&format!("# TYPE {} counter\n", counter.name));
            output.push_str(&format!("{}{{{}}} {}\n", counter.name, labels, counter.value));
        }

        // Gauges
        for (key, value) in &export.gauges {
            // Extract name from key
            if let Some((name, labels)) = key.split_once('{') {
                let labels = labels.trim_end_matches('}');
                output.push_str(&format!("# TYPE {} gauge\n", name));
                output.push_str(&format!("{}_{{{}}} {}\n", name, labels, value));
            }
        }

        // Histograms
        for hist in export.histograms.values() {
            let labels = Self::format_labels(&HashMap::new());
            output.push_str(&format!("# TYPE {} histogram\n", hist.name));
            
            let sum: f64 = hist.values.iter().sum();
            let count = hist.values.len();
            
            output.push_str(&format!("{}_sum{{{}}} {}\n", hist.name, labels, sum));
            output.push_str(&format!("{}_count{{{}}} {}\n", hist.name, labels, count));
        }

        output
    }

    fn make_key(name: &str, labels: &Option<HashMap<String, String>>) -> String {
        match labels {
            Some(l) if !l.is_empty() => {
                let mut parts: Vec<String> = l.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
                parts.sort();
                format!("{}{{{}}}", name, parts.join(","))
            }
            _ => name.into(),
        }
    }

    fn format_labels(labels: &HashMap<String, String>) -> String {
        if labels.is_empty() {
            return String::new();
        }
        let mut parts: Vec<String> = labels.iter().map(|(k, v)| format!("{}=\"{}\"", k, v)).collect();
        parts.sort();
        parts.join(",")
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.counters.write().unwrap().clear();
        self.gauges.write().unwrap().clear();
        self.histograms.write().unwrap().clear();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsExport {
    pub timestamp: String,
    pub uptime_seconds: f64,
    #[serde(default)]
    pub counters: HashMap<String, Counter>,
    #[serde(default)]
    pub gauges: HashMap<String, f64>,
    #[serde(default)]
    pub histograms: HashMap<String, Histogram>,
}

// ─── Predefined Metrics ───────────────────────────────────────────────────────

impl MetricsCollector {
    /// Record session metrics
    pub fn record_session_create(&self) {
        self.increment_counter("openclaw_sessions_created_total", None);
    }

    pub fn record_session_delete(&self) {
        self.increment_counter("openclaw_sessions_deleted_total", None);
    }

    pub fn record_message(&self, direction: &str) {
        let mut labels = HashMap::new();
        labels.insert("direction".into(), direction.into());
        self.increment_counter("openclaw_messages_total", Some(labels));
    }

    /// Record tool metrics
    pub fn record_tool_call(&self, tool_name: &str, success: bool, duration: Duration) {
        let mut labels = HashMap::new();
        labels.insert("tool".into(), tool_name.into());
        labels.insert("status".into(), if success { "success" } else { "error" }.into());
        self.increment_counter("openclaw_tool_calls_total", Some(labels));
        self.record_duration("openclaw_tool_duration_seconds", duration);
    }

    /// Record compaction
    pub fn record_compaction(&self, messages_before: usize, messages_after: usize) {
        let mut labels = HashMap::new();
        labels.insert("messages_before".into(), messages_before.to_string());
        labels.insert("messages_after".into(), messages_after.to_string());
        self.increment_counter("openclaw_compactions_total", Some(labels));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let metrics = MetricsCollector::new();
        metrics.increment_counter("test_counter", None);
        metrics.increment_counter("test_counter", None);
        
        let export = metrics.export_json();
        assert_eq!(export.counters.get("test_counter").unwrap().value, 2);
    }

    #[test]
    fn test_gauge() {
        let metrics = MetricsCollector::new();
        metrics.set_gauge("test_gauge", 42.0, None);
        
        let export = metrics.export_json();
        assert_eq!(export.gauges.get("test_gauge").unwrap(), &42.0);
    }

    #[test]
    fn test_histogram() {
        let metrics = MetricsCollector::new();
        metrics.observe_histogram("test_hist", 1.5, None);
        metrics.observe_histogram("test_hist", 2.5, None);
        
        let export = metrics.export_json();
        let hist = export.histograms.get("test_hist").unwrap();
        assert_eq!(hist.values.len(), 2);
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = MetricsCollector::new();
        metrics.increment_counter("requests_total", None);
        
        let prom = metrics.export_prometheus();
        assert!(prom.contains("requests_total"));
        assert!(prom.contains("# TYPE requests_total counter"));
    }
}
