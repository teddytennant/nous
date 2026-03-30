use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Detected NAT type for a peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NatType {
    /// No NAT — direct public IP.
    None,
    /// Full cone: any external host can send packets to the mapped port.
    FullCone,
    /// Restricted cone: only hosts the internal host has sent to can reply.
    RestrictedCone,
    /// Port-restricted cone: host+port must match for replies.
    PortRestricted,
    /// Symmetric: different mapping for each destination.
    Symmetric,
    /// Not yet determined.
    Unknown,
}

impl NatType {
    /// Whether hole punching is likely to succeed with this NAT type.
    pub fn supports_hole_punch(&self) -> bool {
        matches!(
            self,
            NatType::None | NatType::FullCone | NatType::RestrictedCone | NatType::PortRestricted
        )
    }

    /// Whether a relay is required for connectivity.
    pub fn requires_relay(&self) -> bool {
        matches!(self, NatType::Symmetric)
    }
}

/// An observed external address for the local node.
#[derive(Debug, Clone)]
pub struct ObservedAddress {
    pub address: String,
    /// Number of peers that reported this address.
    pub confirmations: u32,
    /// First time this address was observed.
    pub first_seen: Instant,
    /// Last time this address was observed.
    pub last_seen: Instant,
}

impl ObservedAddress {
    fn new(address: String) -> Self {
        let now = Instant::now();
        Self {
            address,
            confirmations: 1,
            first_seen: now,
            last_seen: now,
        }
    }

    fn confirm(&mut self) {
        self.confirmations += 1;
        self.last_seen = Instant::now();
    }

    /// Address is considered confirmed when multiple peers agree.
    pub fn is_confirmed(&self) -> bool {
        self.confirmations >= 2
    }

    /// Time since this address was last observed.
    pub fn age(&self) -> Duration {
        self.last_seen.elapsed()
    }
}

/// Tracks observed external addresses to determine public IP.
pub struct AddressObserver {
    /// Observed addresses keyed by the address string.
    observed: HashMap<String, ObservedAddress>,
    /// Maximum observations to track.
    max_observations: usize,
    /// How long before an observation is considered stale.
    stale_after: Duration,
}

impl AddressObserver {
    pub fn new(max_observations: usize, stale_after: Duration) -> Self {
        Self {
            observed: HashMap::new(),
            max_observations,
            stale_after,
        }
    }

    /// Record an observed address from a remote peer.
    pub fn observe(&mut self, address: String) {
        if let Some(entry) = self.observed.get_mut(&address) {
            entry.confirm();
        } else {
            if self.observed.len() >= self.max_observations {
                self.gc_stale();
                // If still full, remove the least confirmed.
                if self.observed.len() >= self.max_observations {
                    self.evict_least_confirmed();
                }
            }
            self.observed
                .insert(address.clone(), ObservedAddress::new(address));
        }
    }

    /// Get the most likely external address (highest confirmation count).
    pub fn best_address(&self) -> Option<&ObservedAddress> {
        self.observed
            .values()
            .filter(|a| a.age() < self.stale_after)
            .max_by_key(|a| a.confirmations)
    }

    /// Get all confirmed addresses.
    pub fn confirmed_addresses(&self) -> Vec<&ObservedAddress> {
        self.observed
            .values()
            .filter(|a| a.is_confirmed() && a.age() < self.stale_after)
            .collect()
    }

    /// Get all observed addresses.
    pub fn all_addresses(&self) -> Vec<&ObservedAddress> {
        self.observed.values().collect()
    }

    pub fn observation_count(&self) -> usize {
        self.observed.len()
    }

    /// Remove observations older than the stale threshold.
    pub fn gc_stale(&mut self) {
        let stale_after = self.stale_after;
        self.observed.retain(|_, a| a.age() < stale_after);
    }

    fn evict_least_confirmed(&mut self) {
        if let Some(key) = self
            .observed
            .iter()
            .min_by_key(|(_, a)| a.confirmations)
            .map(|(k, _)| k.clone())
        {
            self.observed.remove(&key);
        }
    }
}

/// Status of a hole punch attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HolePunchStatus {
    /// Waiting for coordination to begin.
    Pending,
    /// Sync messages sent, waiting for reply.
    Initiated,
    /// Connection established via hole punch.
    Success,
    /// Hole punch failed, need relay fallback.
    Failed,
    /// Timed out before completion.
    TimedOut,
}

/// Tracks an in-progress hole punch attempt.
#[derive(Debug, Clone)]
pub struct HolePunchAttempt {
    pub peer_id: String,
    pub peer_addresses: Vec<String>,
    pub status: HolePunchStatus,
    pub started_at: Instant,
    pub attempts: u32,
    pub max_attempts: u32,
    /// Timeout for the entire hole punch process.
    pub timeout: Duration,
}

impl HolePunchAttempt {
    pub fn new(peer_id: String, peer_addresses: Vec<String>) -> Self {
        Self {
            peer_id,
            peer_addresses,
            status: HolePunchStatus::Pending,
            started_at: Instant::now(),
            attempts: 0,
            max_attempts: 3,
            timeout: Duration::from_secs(10),
        }
    }

    /// Start or retry the hole punch.
    pub fn initiate(&mut self) -> bool {
        if self.attempts >= self.max_attempts {
            self.status = HolePunchStatus::Failed;
            return false;
        }
        self.attempts += 1;
        self.status = HolePunchStatus::Initiated;
        true
    }

    /// Mark the hole punch as successful.
    pub fn succeed(&mut self) {
        self.status = HolePunchStatus::Success;
    }

    /// Mark the hole punch as failed.
    pub fn fail(&mut self) {
        self.status = HolePunchStatus::Failed;
    }

    /// Check if the attempt has timed out.
    pub fn is_timed_out(&self) -> bool {
        self.started_at.elapsed() > self.timeout
    }

    /// Whether this attempt can still be retried.
    pub fn can_retry(&self) -> bool {
        self.attempts < self.max_attempts
            && !self.is_timed_out()
            && self.status != HolePunchStatus::Success
    }

    /// Duration since the attempt started.
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }
}

/// Relay node candidate with quality metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayCandidate {
    pub peer_id: String,
    pub address: String,
    /// Measured round-trip latency to the relay.
    pub latency: Option<Duration>,
    /// Whether the relay has been verified as operational.
    pub verified: bool,
    /// Number of times we've successfully used this relay.
    pub usage_count: u64,
    /// Number of failures when trying to use this relay.
    pub failure_count: u64,
    /// Last time this relay was verified.
    #[serde(skip)]
    last_verified: Option<Instant>,
}

impl RelayCandidate {
    pub fn new(peer_id: String, address: String) -> Self {
        Self {
            peer_id,
            address,
            latency: None,
            verified: false,
            usage_count: 0,
            failure_count: 0,
            last_verified: None,
        }
    }

    /// Record a successful relay usage.
    pub fn record_success(&mut self, latency: Duration) {
        self.usage_count += 1;
        self.latency = Some(latency);
        self.verified = true;
        self.last_verified = Some(Instant::now());
    }

    /// Record a relay failure.
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        if self.failure_count > self.usage_count * 2 {
            self.verified = false;
        }
    }

    /// Quality score combining latency and reliability.
    pub fn quality_score(&self) -> f64 {
        let total = self.usage_count + self.failure_count;
        if total == 0 {
            return 0.0;
        }

        let reliability = self.usage_count as f64 / total as f64;
        let latency_score = match self.latency {
            Some(d) if d.as_millis() < 50 => 1.0,
            Some(d) if d.as_millis() < 150 => 0.8,
            Some(d) if d.as_millis() < 500 => 0.5,
            Some(_) => 0.2,
            None => 0.1,
        };

        reliability * 0.7 + latency_score * 0.3
    }

    /// Whether the relay verification is still fresh.
    pub fn is_fresh(&self, max_age: Duration) -> bool {
        self.last_verified.is_some_and(|t| t.elapsed() < max_age)
    }
}

/// Manages relay selection and fallback for peers behind NAT.
pub struct RelaySelector {
    candidates: Vec<RelayCandidate>,
    /// Currently active relay, if any.
    active_relay: Option<String>,
    max_candidates: usize,
}

impl RelaySelector {
    pub fn new(max_candidates: usize) -> Self {
        Self {
            candidates: Vec::new(),
            active_relay: None,
            max_candidates,
        }
    }

    /// Add a relay candidate.
    pub fn add_candidate(&mut self, candidate: RelayCandidate) {
        if self
            .candidates
            .iter()
            .any(|c| c.peer_id == candidate.peer_id)
        {
            return;
        }
        if self.candidates.len() >= self.max_candidates {
            self.evict_worst();
        }
        self.candidates.push(candidate);
    }

    /// Select the best relay based on quality score.
    pub fn select_best(&mut self) -> Option<&RelayCandidate> {
        if self.candidates.is_empty() {
            return None;
        }

        self.candidates
            .sort_by(|a, b| b.quality_score().partial_cmp(&a.quality_score()).unwrap());

        let best = &self.candidates[0];
        self.active_relay = Some(best.peer_id.clone());
        Some(best)
    }

    /// Get the currently active relay.
    pub fn active_relay(&self) -> Option<&RelayCandidate> {
        self.active_relay
            .as_ref()
            .and_then(|active| self.candidates.iter().find(|c| c.peer_id == *active))
    }

    /// Record success for a relay.
    pub fn record_success(&mut self, peer_id: &str, latency: Duration) {
        if let Some(c) = self.candidates.iter_mut().find(|c| c.peer_id == peer_id) {
            c.record_success(latency);
        }
    }

    /// Record failure for a relay, potentially triggering failover.
    pub fn record_failure(&mut self, peer_id: &str) -> bool {
        if let Some(c) = self.candidates.iter_mut().find(|c| c.peer_id == peer_id) {
            c.record_failure();
        }

        // If the failed relay was active, clear it to trigger re-selection.
        let need_failover = self
            .active_relay
            .as_ref()
            .is_some_and(|active| active == peer_id);
        if need_failover {
            self.active_relay = None;
        }
        need_failover
    }

    /// Get all verified relays sorted by quality.
    pub fn verified_relays(&self) -> Vec<&RelayCandidate> {
        let mut verified: Vec<&RelayCandidate> =
            self.candidates.iter().filter(|c| c.verified).collect();
        verified.sort_by(|a, b| b.quality_score().partial_cmp(&a.quality_score()).unwrap());
        verified
    }

    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    fn evict_worst(&mut self) {
        if self.candidates.is_empty() {
            return;
        }
        let worst_idx = self
            .candidates
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.quality_score().partial_cmp(&b.quality_score()).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.candidates.remove(worst_idx);
    }
}

/// Comprehensive NAT traversal coordinator.
///
/// Combines address observation, NAT type detection, hole punching,
/// and relay fallback into a unified traversal strategy.
pub struct NatTraversal {
    pub nat_type: NatType,
    address_observer: AddressObserver,
    relay_selector: RelaySelector,
    active_punches: HashMap<String, HolePunchAttempt>,
    max_concurrent_punches: usize,
}

impl NatTraversal {
    pub fn new() -> Self {
        Self {
            nat_type: NatType::Unknown,
            address_observer: AddressObserver::new(50, Duration::from_secs(600)),
            relay_selector: RelaySelector::new(20),
            active_punches: HashMap::new(),
            max_concurrent_punches: 5,
        }
    }

    /// Observe an external address reported by a peer.
    pub fn observe_address(&mut self, address: String) {
        self.address_observer.observe(address);
    }

    /// Get our best known external address.
    pub fn external_address(&self) -> Option<&str> {
        self.address_observer
            .best_address()
            .map(|a| a.address.as_str())
    }

    /// Set the detected NAT type.
    pub fn set_nat_type(&mut self, nat_type: NatType) {
        self.nat_type = nat_type;
    }

    /// Determine the best connectivity strategy for a peer.
    pub fn strategy_for(&self, peer_nat: NatType) -> ConnectivityStrategy {
        match (self.nat_type, peer_nat) {
            (NatType::None, _) | (_, NatType::None) => ConnectivityStrategy::Direct,
            (NatType::FullCone, _) | (_, NatType::FullCone) => ConnectivityStrategy::Direct,
            (NatType::Symmetric, NatType::Symmetric) => ConnectivityStrategy::Relay,
            (NatType::Symmetric, _) | (_, NatType::Symmetric) => ConnectivityStrategy::Relay,
            _ => ConnectivityStrategy::HolePunch,
        }
    }

    /// Start a hole punch attempt to a peer.
    pub fn start_hole_punch(
        &mut self,
        peer_id: String,
        peer_addresses: Vec<String>,
    ) -> Result<&HolePunchAttempt, &'static str> {
        if self.active_punches.len() >= self.max_concurrent_punches {
            return Err("too many concurrent hole punch attempts");
        }

        if self.active_punches.contains_key(&peer_id) {
            return Err("hole punch already in progress for this peer");
        }

        let mut attempt = HolePunchAttempt::new(peer_id.clone(), peer_addresses);
        attempt.initiate();
        self.active_punches.insert(peer_id.clone(), attempt);
        Ok(self.active_punches.get(&peer_id).unwrap())
    }

    /// Mark a hole punch as successful.
    pub fn hole_punch_succeeded(&mut self, peer_id: &str) {
        if let Some(attempt) = self.active_punches.get_mut(peer_id) {
            attempt.succeed();
        }
    }

    /// Mark a hole punch as failed.
    pub fn hole_punch_failed(&mut self, peer_id: &str) {
        if let Some(attempt) = self.active_punches.get_mut(peer_id) {
            if attempt.can_retry() {
                attempt.initiate();
            } else {
                attempt.fail();
            }
        }
    }

    /// Get the status of a hole punch attempt.
    pub fn hole_punch_status(&self, peer_id: &str) -> Option<HolePunchStatus> {
        self.active_punches.get(peer_id).map(|a| a.status)
    }

    /// Clean up completed and timed-out hole punch attempts.
    pub fn gc_punches(&mut self) {
        self.active_punches.retain(|_, attempt| {
            if attempt.is_timed_out() && attempt.status != HolePunchStatus::Success {
                return false;
            }
            matches!(
                attempt.status,
                HolePunchStatus::Pending | HolePunchStatus::Initiated
            )
        });
    }

    /// Add a relay candidate.
    pub fn add_relay(&mut self, candidate: RelayCandidate) {
        self.relay_selector.add_candidate(candidate);
    }

    /// Select the best relay for connectivity.
    pub fn select_relay(&mut self) -> Option<&RelayCandidate> {
        self.relay_selector.select_best()
    }

    /// Record relay success.
    pub fn relay_succeeded(&mut self, peer_id: &str, latency: Duration) {
        self.relay_selector.record_success(peer_id, latency);
    }

    /// Record relay failure.
    pub fn relay_failed(&mut self, peer_id: &str) -> bool {
        self.relay_selector.record_failure(peer_id)
    }

    /// Get active relay.
    pub fn active_relay(&self) -> Option<&RelayCandidate> {
        self.relay_selector.active_relay()
    }

    pub fn active_punch_count(&self) -> usize {
        self.active_punches.len()
    }
}

impl Default for NatTraversal {
    fn default() -> Self {
        Self::new()
    }
}

/// Recommended connectivity strategy based on NAT types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectivityStrategy {
    /// Direct connection possible.
    Direct,
    /// Hole punching likely to succeed.
    HolePunch,
    /// Relay required.
    Relay,
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- NatType tests ---

    #[test]
    fn nat_type_hole_punch_support() {
        assert!(NatType::None.supports_hole_punch());
        assert!(NatType::FullCone.supports_hole_punch());
        assert!(NatType::RestrictedCone.supports_hole_punch());
        assert!(NatType::PortRestricted.supports_hole_punch());
        assert!(!NatType::Symmetric.supports_hole_punch());
        assert!(!NatType::Unknown.supports_hole_punch());
    }

    #[test]
    fn nat_type_relay_requirement() {
        assert!(NatType::Symmetric.requires_relay());
        assert!(!NatType::None.requires_relay());
        assert!(!NatType::FullCone.requires_relay());
    }

    // --- AddressObserver tests ---

    #[test]
    fn observer_tracks_addresses() {
        let mut obs = AddressObserver::new(10, Duration::from_secs(300));
        obs.observe("1.2.3.4:9000".into());
        assert_eq!(obs.observation_count(), 1);

        let addr = obs.best_address().unwrap();
        assert_eq!(addr.address, "1.2.3.4:9000");
        assert_eq!(addr.confirmations, 1);
    }

    #[test]
    fn observer_confirms_address() {
        let mut obs = AddressObserver::new(10, Duration::from_secs(300));
        obs.observe("1.2.3.4:9000".into());
        obs.observe("1.2.3.4:9000".into());
        obs.observe("1.2.3.4:9000".into());

        let addr = obs.best_address().unwrap();
        assert_eq!(addr.confirmations, 3);
        assert!(addr.is_confirmed());
    }

    #[test]
    fn observer_prefers_most_confirmed() {
        let mut obs = AddressObserver::new(10, Duration::from_secs(300));
        obs.observe("1.1.1.1:9000".into());
        obs.observe("2.2.2.2:9000".into());
        obs.observe("2.2.2.2:9000".into());
        obs.observe("2.2.2.2:9000".into());

        let best = obs.best_address().unwrap();
        assert_eq!(best.address, "2.2.2.2:9000");
    }

    #[test]
    fn observer_evicts_least_confirmed_when_full() {
        let mut obs = AddressObserver::new(2, Duration::from_secs(300));
        obs.observe("1.1.1.1".into());
        obs.observe("2.2.2.2".into());
        obs.observe("2.2.2.2".into()); // Confirm 2.2.2.2
        obs.observe("3.3.3.3".into()); // Should evict 1.1.1.1 (least confirmed)

        assert_eq!(obs.observation_count(), 2);
        assert!(
            obs.confirmed_addresses()
                .iter()
                .any(|a| a.address == "2.2.2.2")
        );
    }

    // --- HolePunchAttempt tests ---

    #[test]
    fn hole_punch_lifecycle() {
        let mut attempt = HolePunchAttempt::new("peer-a".into(), vec!["/ip4/1.1.1.1".into()]);
        assert_eq!(attempt.status, HolePunchStatus::Pending);

        assert!(attempt.initiate());
        assert_eq!(attempt.status, HolePunchStatus::Initiated);
        assert_eq!(attempt.attempts, 1);

        attempt.succeed();
        assert_eq!(attempt.status, HolePunchStatus::Success);
    }

    #[test]
    fn hole_punch_max_attempts() {
        let mut attempt = HolePunchAttempt::new("peer-a".into(), vec![]);
        attempt.max_attempts = 2;

        assert!(attempt.initiate()); // Attempt 1
        assert!(attempt.initiate()); // Attempt 2
        assert!(!attempt.initiate()); // Exceeds max
        assert_eq!(attempt.status, HolePunchStatus::Failed);
    }

    #[test]
    fn hole_punch_timeout() {
        let mut attempt = HolePunchAttempt::new("peer-a".into(), vec![]);
        attempt.timeout = Duration::from_millis(10);
        std::thread::sleep(Duration::from_millis(20));
        assert!(attempt.is_timed_out());
        assert!(!attempt.can_retry());
    }

    // --- RelayCandidate tests ---

    #[test]
    fn relay_quality_score() {
        let mut relay = RelayCandidate::new("relay-1".into(), "/ip4/1.1.1.1".into());

        // No data → 0 score.
        assert_eq!(relay.quality_score(), 0.0);

        // All successes with low latency → high score.
        for _ in 0..10 {
            relay.record_success(Duration::from_millis(30));
        }
        assert!(relay.quality_score() > 0.9);
    }

    #[test]
    fn relay_quality_degrades_with_failures() {
        let mut relay = RelayCandidate::new("relay-1".into(), "/ip4/1.1.1.1".into());
        relay.record_success(Duration::from_millis(30));
        let good_score = relay.quality_score();

        relay.record_failure();
        relay.record_failure();
        relay.record_failure();
        assert!(relay.quality_score() < good_score);
    }

    #[test]
    fn relay_verification_lost_on_many_failures() {
        let mut relay = RelayCandidate::new("relay-1".into(), "/ip4/1.1.1.1".into());
        relay.record_success(Duration::from_millis(30));
        assert!(relay.verified);

        // Failures > 2x successes → lose verification.
        relay.record_failure();
        relay.record_failure();
        relay.record_failure();
        assert!(!relay.verified);
    }

    // --- RelaySelector tests ---

    #[test]
    fn relay_selector_selects_best() {
        let mut selector = RelaySelector::new(10);

        let mut good = RelayCandidate::new("good".into(), "/ip4/1.1.1.1".into());
        good.record_success(Duration::from_millis(20));
        good.record_success(Duration::from_millis(25));

        let mut bad = RelayCandidate::new("bad".into(), "/ip4/2.2.2.2".into());
        bad.record_failure();

        selector.add_candidate(bad);
        selector.add_candidate(good);

        let best = selector.select_best().unwrap();
        assert_eq!(best.peer_id, "good");
        assert_eq!(selector.active_relay().unwrap().peer_id, "good");
    }

    #[test]
    fn relay_selector_failover() {
        let mut selector = RelaySelector::new(10);

        let mut r1 = RelayCandidate::new("r1".into(), "/ip4/1.1.1.1".into());
        r1.record_success(Duration::from_millis(20));
        selector.add_candidate(r1);

        selector.select_best();
        assert_eq!(selector.active_relay().unwrap().peer_id, "r1");

        // Failure on active relay triggers failover.
        let need_failover = selector.record_failure("r1");
        assert!(need_failover);
        assert!(selector.active_relay().is_none());
    }

    #[test]
    fn relay_selector_no_duplicate_candidates() {
        let mut selector = RelaySelector::new(10);
        selector.add_candidate(RelayCandidate::new("r1".into(), "/ip4/1.1.1.1".into()));
        selector.add_candidate(RelayCandidate::new("r1".into(), "/ip4/1.1.1.1".into()));
        assert_eq!(selector.candidate_count(), 1);
    }

    // --- NatTraversal tests ---

    #[test]
    fn nat_traversal_strategy_both_public() {
        let t = NatTraversal::new();
        let mut t = t;
        t.set_nat_type(NatType::None);
        assert_eq!(t.strategy_for(NatType::None), ConnectivityStrategy::Direct);
    }

    #[test]
    fn nat_traversal_strategy_symmetric_needs_relay() {
        let mut t = NatTraversal::new();
        t.set_nat_type(NatType::Symmetric);
        assert_eq!(
            t.strategy_for(NatType::Symmetric),
            ConnectivityStrategy::Relay
        );
    }

    #[test]
    fn nat_traversal_strategy_cone_can_hole_punch() {
        let mut t = NatTraversal::new();
        t.set_nat_type(NatType::RestrictedCone);
        assert_eq!(
            t.strategy_for(NatType::PortRestricted),
            ConnectivityStrategy::HolePunch
        );
    }

    #[test]
    fn nat_traversal_hole_punch_flow() {
        let mut t = NatTraversal::new();
        t.start_hole_punch("peer-a".into(), vec!["/ip4/1.1.1.1".into()])
            .unwrap();

        assert_eq!(
            t.hole_punch_status("peer-a"),
            Some(HolePunchStatus::Initiated)
        );

        t.hole_punch_succeeded("peer-a");
        assert_eq!(
            t.hole_punch_status("peer-a"),
            Some(HolePunchStatus::Success)
        );
    }

    #[test]
    fn nat_traversal_rejects_duplicate_punch() {
        let mut t = NatTraversal::new();
        t.start_hole_punch("peer-a".into(), vec![]).unwrap();
        assert!(t.start_hole_punch("peer-a".into(), vec![]).is_err());
    }

    #[test]
    fn nat_traversal_max_concurrent_punches() {
        let mut t = NatTraversal::new();
        t.max_concurrent_punches = 2;

        t.start_hole_punch("p1".into(), vec![]).unwrap();
        t.start_hole_punch("p2".into(), vec![]).unwrap();
        assert!(t.start_hole_punch("p3".into(), vec![]).is_err());
    }

    #[test]
    fn nat_traversal_external_address() {
        let mut t = NatTraversal::new();
        t.observe_address("1.2.3.4:9000".into());
        t.observe_address("1.2.3.4:9000".into());

        assert_eq!(t.external_address(), Some("1.2.3.4:9000"));
    }

    #[test]
    fn nat_traversal_relay_integration() {
        let mut t = NatTraversal::new();
        let mut relay = RelayCandidate::new("relay-1".into(), "/ip4/10.0.0.1".into());
        relay.record_success(Duration::from_millis(30));
        t.add_relay(relay);

        let selected = t.select_relay().unwrap();
        assert_eq!(selected.peer_id, "relay-1");
        assert_eq!(t.active_relay().unwrap().peer_id, "relay-1");
    }

    #[test]
    fn nat_type_serializes() {
        let nat = NatType::PortRestricted;
        let json = serde_json::to_string(&nat).unwrap();
        let decoded: NatType = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, nat);
    }

    #[test]
    fn connectivity_strategy_serializes() {
        let strategy = ConnectivityStrategy::HolePunch;
        let json = serde_json::to_string(&strategy).unwrap();
        let decoded: ConnectivityStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, strategy);
    }
}
