//! Node health monitoring: subsystem status, uptime tracking, and metrics.
//!
//! Provides a unified view of node health for dashboards, API endpoints,
//! and automated alerting.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Overall node health status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// All subsystems healthy.
    Healthy,
    /// Some subsystems degraded but node is functional.
    Degraded,
    /// Critical failure — node cannot serve requests.
    Unhealthy,
}

/// Status of an individual subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubsystemStatus {
    Up,
    Degraded,
    Down,
    Unknown,
}

/// Health information for a single subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemHealth {
    pub name: String,
    pub status: SubsystemStatus,
    pub message: Option<String>,
    #[serde(skip)]
    pub last_check: Option<Instant>,
    pub check_count: u64,
    pub failure_count: u64,
}

impl SubsystemHealth {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: SubsystemStatus::Unknown,
            message: None,
            last_check: None,
            check_count: 0,
            failure_count: 0,
        }
    }

    pub fn mark_healthy(&mut self, message: Option<&str>) {
        self.status = SubsystemStatus::Up;
        self.message = message.map(|s| s.to_string());
        self.last_check = Some(Instant::now());
        self.check_count += 1;
    }

    pub fn mark_degraded(&mut self, message: &str) {
        self.status = SubsystemStatus::Degraded;
        self.message = Some(message.to_string());
        self.last_check = Some(Instant::now());
        self.check_count += 1;
    }

    pub fn mark_down(&mut self, message: &str) {
        self.status = SubsystemStatus::Down;
        self.message = Some(message.to_string());
        self.last_check = Some(Instant::now());
        self.check_count += 1;
        self.failure_count += 1;
    }

    /// Time since last health check.
    pub fn since_last_check(&self) -> Option<Duration> {
        self.last_check.map(|t| t.elapsed())
    }

    /// Failure rate as a fraction (0.0 to 1.0).
    pub fn failure_rate(&self) -> f64 {
        if self.check_count == 0 {
            return 0.0;
        }
        self.failure_count as f64 / self.check_count as f64
    }
}

/// A gauge metric (current value).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gauge {
    pub name: String,
    pub value: f64,
    pub unit: String,
}

/// A counter metric (monotonically increasing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Counter {
    pub name: String,
    pub value: u64,
}

/// Collected node metrics.
#[derive(Debug, Default)]
pub struct NodeMetrics {
    gauges: HashMap<String, Gauge>,
    counters: HashMap<String, Counter>,
}

impl NodeMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_gauge(&mut self, name: &str, value: f64, unit: &str) {
        self.gauges.insert(
            name.to_string(),
            Gauge {
                name: name.to_string(),
                value,
                unit: unit.to_string(),
            },
        );
    }

    pub fn get_gauge(&self, name: &str) -> Option<f64> {
        self.gauges.get(name).map(|g| g.value)
    }

    pub fn increment_counter(&mut self, name: &str) {
        self.counters
            .entry(name.to_string())
            .and_modify(|c| c.value += 1)
            .or_insert(Counter {
                name: name.to_string(),
                value: 1,
            });
    }

    pub fn increment_counter_by(&mut self, name: &str, amount: u64) {
        self.counters
            .entry(name.to_string())
            .and_modify(|c| c.value += amount)
            .or_insert(Counter {
                name: name.to_string(),
                value: amount,
            });
    }

    pub fn get_counter(&self, name: &str) -> u64 {
        self.counters.get(name).map(|c| c.value).unwrap_or(0)
    }

    pub fn all_gauges(&self) -> Vec<&Gauge> {
        self.gauges.values().collect()
    }

    pub fn all_counters(&self) -> Vec<&Counter> {
        self.counters.values().collect()
    }
}

/// The health monitor for a Nous node.
#[derive(Debug)]
pub struct HealthMonitor {
    subsystems: HashMap<String, SubsystemHealth>,
    started_at: Instant,
    metrics: NodeMetrics,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            subsystems: HashMap::new(),
            started_at: Instant::now(),
            metrics: NodeMetrics::new(),
        }
    }

    /// Register a subsystem for monitoring.
    pub fn register(&mut self, name: &str) {
        self.subsystems
            .entry(name.to_string())
            .or_insert_with(|| SubsystemHealth::new(name));
    }

    /// Get a subsystem's health.
    pub fn get(&self, name: &str) -> Option<&SubsystemHealth> {
        self.subsystems.get(name)
    }

    /// Get a mutable reference to a subsystem's health.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut SubsystemHealth> {
        self.subsystems.get_mut(name)
    }

    /// Overall health status. Unhealthy if any critical subsystem is down.
    /// Degraded if any subsystem is degraded or non-critical is down.
    pub fn overall_status(&self) -> HealthStatus {
        let mut has_degraded = false;
        for sub in self.subsystems.values() {
            match sub.status {
                SubsystemStatus::Down => return HealthStatus::Unhealthy,
                SubsystemStatus::Degraded => has_degraded = true,
                SubsystemStatus::Unknown => has_degraded = true,
                SubsystemStatus::Up => {}
            }
        }
        if has_degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }

    /// Node uptime.
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Access metrics.
    pub fn metrics(&self) -> &NodeMetrics {
        &self.metrics
    }

    /// Access metrics mutably.
    pub fn metrics_mut(&mut self) -> &mut NodeMetrics {
        &mut self.metrics
    }

    /// All subsystem names.
    pub fn subsystem_names(&self) -> Vec<&str> {
        self.subsystems.keys().map(|s| s.as_str()).collect()
    }

    /// Number of registered subsystems.
    pub fn subsystem_count(&self) -> usize {
        self.subsystems.len()
    }

    /// Generate a health report snapshot.
    pub fn report(&self) -> HealthReport {
        let subsystems: Vec<SubsystemReport> = self
            .subsystems
            .values()
            .map(|s| SubsystemReport {
                name: s.name.clone(),
                status: s.status,
                message: s.message.clone(),
                failure_rate: s.failure_rate(),
            })
            .collect();

        HealthReport {
            status: self.overall_status(),
            uptime_secs: self.uptime().as_secs(),
            subsystems,
        }
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializable health report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub status: HealthStatus,
    pub uptime_secs: u64,
    pub subsystems: Vec<SubsystemReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemReport {
    pub name: String,
    pub status: SubsystemStatus,
    pub message: Option<String>,
    pub failure_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SubsystemHealth ────────────────────────────────────────

    #[test]
    fn subsystem_starts_unknown() {
        let sub = SubsystemHealth::new("network");
        assert_eq!(sub.status, SubsystemStatus::Unknown);
        assert_eq!(sub.check_count, 0);
        assert!(sub.since_last_check().is_none());
    }

    #[test]
    fn mark_healthy() {
        let mut sub = SubsystemHealth::new("storage");
        sub.mark_healthy(Some("all good"));
        assert_eq!(sub.status, SubsystemStatus::Up);
        assert_eq!(sub.message.as_deref(), Some("all good"));
        assert_eq!(sub.check_count, 1);
        assert!(sub.since_last_check().is_some());
    }

    #[test]
    fn mark_degraded() {
        let mut sub = SubsystemHealth::new("network");
        sub.mark_degraded("high latency");
        assert_eq!(sub.status, SubsystemStatus::Degraded);
        assert_eq!(sub.message.as_deref(), Some("high latency"));
    }

    #[test]
    fn mark_down_increments_failures() {
        let mut sub = SubsystemHealth::new("storage");
        sub.mark_healthy(None);
        sub.mark_down("disk full");
        assert_eq!(sub.status, SubsystemStatus::Down);
        assert_eq!(sub.failure_count, 1);
        assert_eq!(sub.check_count, 2);
    }

    #[test]
    fn failure_rate_zero_when_no_checks() {
        let sub = SubsystemHealth::new("test");
        assert_eq!(sub.failure_rate(), 0.0);
    }

    #[test]
    fn failure_rate_computes_correctly() {
        let mut sub = SubsystemHealth::new("test");
        sub.mark_healthy(None);
        sub.mark_healthy(None);
        sub.mark_down("fail");
        sub.mark_healthy(None);
        assert!((sub.failure_rate() - 0.25).abs() < f64::EPSILON);
    }

    // ── NodeMetrics ────────────────────────────────────────────

    #[test]
    fn gauge_set_and_get() {
        let mut metrics = NodeMetrics::new();
        metrics.set_gauge("memory_mb", 512.0, "MB");
        assert_eq!(metrics.get_gauge("memory_mb"), Some(512.0));
    }

    #[test]
    fn gauge_overwrite() {
        let mut metrics = NodeMetrics::new();
        metrics.set_gauge("cpu", 50.0, "%");
        metrics.set_gauge("cpu", 75.0, "%");
        assert_eq!(metrics.get_gauge("cpu"), Some(75.0));
    }

    #[test]
    fn counter_increment() {
        let mut metrics = NodeMetrics::new();
        metrics.increment_counter("requests");
        metrics.increment_counter("requests");
        metrics.increment_counter("requests");
        assert_eq!(metrics.get_counter("requests"), 3);
    }

    #[test]
    fn counter_increment_by() {
        let mut metrics = NodeMetrics::new();
        metrics.increment_counter_by("bytes_sent", 1024);
        metrics.increment_counter_by("bytes_sent", 2048);
        assert_eq!(metrics.get_counter("bytes_sent"), 3072);
    }

    #[test]
    fn missing_gauge_returns_none() {
        let metrics = NodeMetrics::new();
        assert!(metrics.get_gauge("nonexistent").is_none());
    }

    #[test]
    fn missing_counter_returns_zero() {
        let metrics = NodeMetrics::new();
        assert_eq!(metrics.get_counter("nonexistent"), 0);
    }

    // ── HealthMonitor ──────────────────────────────────────────

    #[test]
    fn monitor_starts_healthy() {
        let monitor = HealthMonitor::new();
        assert_eq!(monitor.overall_status(), HealthStatus::Healthy);
        assert_eq!(monitor.subsystem_count(), 0);
    }

    #[test]
    fn register_and_check_subsystem() {
        let mut monitor = HealthMonitor::new();
        monitor.register("network");
        monitor.register("storage");

        assert_eq!(monitor.subsystem_count(), 2);
        assert!(monitor.get("network").is_some());
    }

    #[test]
    fn overall_healthy_when_all_up() {
        let mut monitor = HealthMonitor::new();
        monitor.register("network");
        monitor.register("storage");

        monitor.get_mut("network").unwrap().mark_healthy(None);
        monitor.get_mut("storage").unwrap().mark_healthy(None);

        assert_eq!(monitor.overall_status(), HealthStatus::Healthy);
    }

    #[test]
    fn overall_degraded_when_subsystem_degraded() {
        let mut monitor = HealthMonitor::new();
        monitor.register("network");
        monitor.register("storage");

        monitor.get_mut("network").unwrap().mark_healthy(None);
        monitor.get_mut("storage").unwrap().mark_degraded("slow");

        assert_eq!(monitor.overall_status(), HealthStatus::Degraded);
    }

    #[test]
    fn overall_unhealthy_when_subsystem_down() {
        let mut monitor = HealthMonitor::new();
        monitor.register("network");
        monitor.register("storage");

        monitor.get_mut("network").unwrap().mark_healthy(None);
        monitor.get_mut("storage").unwrap().mark_down("crashed");

        assert_eq!(monitor.overall_status(), HealthStatus::Unhealthy);
    }

    #[test]
    fn overall_degraded_when_unknown() {
        let mut monitor = HealthMonitor::new();
        monitor.register("network");
        // Unknown status = degraded overall
        assert_eq!(monitor.overall_status(), HealthStatus::Degraded);
    }

    #[test]
    fn uptime_increases() {
        let monitor = HealthMonitor::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(monitor.uptime().as_millis() >= 10);
    }

    #[test]
    fn monitor_metrics() {
        let mut monitor = HealthMonitor::new();
        monitor.metrics_mut().set_gauge("peers", 5.0, "count");
        monitor.metrics_mut().increment_counter("messages");

        assert_eq!(monitor.metrics().get_gauge("peers"), Some(5.0));
        assert_eq!(monitor.metrics().get_counter("messages"), 1);
    }

    #[test]
    fn health_report_serializes() {
        let mut monitor = HealthMonitor::new();
        monitor.register("network");
        monitor.get_mut("network").unwrap().mark_healthy(None);

        let report = monitor.report();
        let json = serde_json::to_string(&report).unwrap();
        let restored: HealthReport = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.status, HealthStatus::Healthy);
        assert_eq!(restored.subsystems.len(), 1);
    }

    #[test]
    fn report_contains_all_subsystems() {
        let mut monitor = HealthMonitor::new();
        monitor.register("network");
        monitor.register("storage");
        monitor.register("identity");

        let report = monitor.report();
        assert_eq!(report.subsystems.len(), 3);
    }

    #[test]
    fn duplicate_register_is_noop() {
        let mut monitor = HealthMonitor::new();
        monitor.register("network");
        monitor.get_mut("network").unwrap().mark_healthy(None);
        monitor.register("network"); // should not reset

        assert_eq!(monitor.subsystem_count(), 1);
        assert_eq!(monitor.get("network").unwrap().status, SubsystemStatus::Up);
    }
}
