//! Payment streams: continuous micropayment flows.
//!
//! A stream authorizes a per-second token flow from payer to payee.
//! The payee periodically claims accumulated value. Streams can be
//! paused, resumed, topped up, and drained. Useful for paying for
//! compute, bandwidth, storage, or any metered service.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::{Error, Result};

/// Stream lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamState {
    /// Created but not yet funded.
    Pending,
    /// Actively streaming tokens.
    Active,
    /// Temporarily paused (can be resumed).
    Paused,
    /// Fully drained or cancelled. Terminal.
    Closed,
}

/// A payment stream configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Tokens per second (in smallest denomination).
    pub rate_per_second: u128,
    /// Token identifier (e.g. "ETH", "USDC").
    pub token: String,
    /// If true, payee can claim at any time. If false, claims happen at intervals.
    pub continuous_claim: bool,
    /// Minimum seconds between claims (0 for unlimited).
    pub min_claim_interval_secs: u64,
    /// Maximum stream duration in seconds (0 for unlimited).
    pub max_duration_secs: u64,
}

/// A claim receipt for withdrawn stream funds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimReceipt {
    pub stream_id: String,
    pub amount: u128,
    pub claimed_at: DateTime<Utc>,
    pub elapsed_secs: u64,
    pub remaining_deposit: u128,
}

/// A continuous payment stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentStream {
    pub id: String,
    pub payer: String,
    pub payee: String,
    pub config: StreamConfig,
    pub state: StreamState,
    /// Total deposited by the payer.
    pub total_deposited: u128,
    /// Total already claimed by the payee.
    pub total_claimed: u128,
    /// When the stream was created.
    pub created_at: DateTime<Utc>,
    /// When the stream started (first activation).
    pub started_at: Option<DateTime<Utc>>,
    /// When the stream was last paused (for accounting).
    pub paused_at: Option<DateTime<Utc>>,
    /// Cumulative seconds the stream was active before the latest pause.
    pub active_seconds_before_pause: u64,
    /// Last claim timestamp.
    pub last_claimed_at: Option<DateTime<Utc>>,
    /// Claim history.
    pub claims: Vec<ClaimReceipt>,
}

impl PaymentStream {
    /// Create a new payment stream.
    pub fn create(
        payer: &str,
        payee: &str,
        config: StreamConfig,
        initial_deposit: u128,
    ) -> Result<Self> {
        if payer == payee {
            return Err(Error::InvalidInput("payer and payee must differ".into()));
        }
        if config.rate_per_second == 0 {
            return Err(Error::InvalidInput("rate must be positive".into()));
        }
        if initial_deposit == 0 {
            return Err(Error::InvalidInput("deposit must be positive".into()));
        }

        Ok(Self {
            id: format!("stream:{}", Uuid::new_v4()),
            payer: payer.into(),
            payee: payee.into(),
            config,
            state: StreamState::Pending,
            total_deposited: initial_deposit,
            total_claimed: 0,
            created_at: Utc::now(),
            started_at: None,
            paused_at: None,
            active_seconds_before_pause: 0,
            last_claimed_at: None,
            claims: Vec::new(),
        })
    }

    /// Activate the stream and start flowing tokens.
    pub fn activate(&mut self) -> Result<()> {
        if self.state != StreamState::Pending {
            return Err(Error::InvalidInput("stream is not pending".into()));
        }
        let now = Utc::now();
        self.state = StreamState::Active;
        self.started_at = Some(now);
        Ok(())
    }

    /// Activate the stream at a specific time (for deterministic testing).
    pub fn activate_at(&mut self, at: DateTime<Utc>) -> Result<()> {
        if self.state != StreamState::Pending {
            return Err(Error::InvalidInput("stream is not pending".into()));
        }
        self.state = StreamState::Active;
        self.started_at = Some(at);
        Ok(())
    }

    /// Pause the stream. Tokens stop flowing.
    pub fn pause(&mut self) -> Result<()> {
        self.pause_at(Utc::now())
    }

    /// Pause at a specific time (for testing).
    pub fn pause_at(&mut self, at: DateTime<Utc>) -> Result<()> {
        if self.state != StreamState::Active {
            return Err(Error::InvalidInput("stream is not active".into()));
        }
        self.active_seconds_before_pause += self.active_seconds_since_last_resume(at);
        self.paused_at = Some(at);
        self.state = StreamState::Paused;
        Ok(())
    }

    /// Resume a paused stream.
    pub fn resume(&mut self) -> Result<()> {
        self.resume_at(Utc::now())
    }

    /// Resume at a specific time (for testing).
    pub fn resume_at(&mut self, at: DateTime<Utc>) -> Result<()> {
        if self.state != StreamState::Paused {
            return Err(Error::InvalidInput("stream is not paused".into()));
        }
        self.state = StreamState::Active;
        self.paused_at = None;
        // Update started_at to the resume time so active_seconds_since_last_resume works
        self.started_at = Some(at);
        Ok(())
    }

    /// Top up the stream's deposit.
    pub fn top_up(&mut self, amount: u128) -> Result<()> {
        if self.state == StreamState::Closed {
            return Err(Error::InvalidInput("stream is closed".into()));
        }
        if amount == 0 {
            return Err(Error::InvalidInput("amount must be positive".into()));
        }
        self.total_deposited += amount;
        Ok(())
    }

    /// Calculate total active seconds up to a given time.
    pub fn total_active_seconds(&self, now: DateTime<Utc>) -> u64 {
        match self.state {
            StreamState::Active => {
                self.active_seconds_before_pause + self.active_seconds_since_last_resume(now)
            }
            StreamState::Paused | StreamState::Closed => self.active_seconds_before_pause,
            StreamState::Pending => 0,
        }
    }

    /// Total value that has streamed (regardless of claims).
    pub fn total_streamed(&self, now: DateTime<Utc>) -> u128 {
        let secs = self.total_active_seconds(now) as u128;
        let raw = secs * self.config.rate_per_second;
        // Cap at deposit
        raw.min(self.total_deposited)
    }

    /// Claimable amount at a given time.
    pub fn claimable(&self, now: DateTime<Utc>) -> u128 {
        let streamed = self.total_streamed(now);
        streamed.saturating_sub(self.total_claimed)
    }

    /// Remaining deposit (not yet streamed).
    pub fn remaining_deposit(&self, now: DateTime<Utc>) -> u128 {
        self.total_deposited
            .saturating_sub(self.total_streamed(now))
    }

    /// Time until the stream runs out of funds at the current rate.
    pub fn time_to_depletion(&self, now: DateTime<Utc>) -> Option<Duration> {
        if self.state != StreamState::Active || self.config.rate_per_second == 0 {
            return None;
        }
        let remaining = self.remaining_deposit(now);
        if remaining == 0 {
            return Some(Duration::zero());
        }
        let secs = remaining / self.config.rate_per_second;
        Some(Duration::seconds(secs as i64))
    }

    /// Claim accumulated tokens. Returns a receipt.
    pub fn claim(&mut self) -> Result<ClaimReceipt> {
        self.claim_at(Utc::now())
    }

    /// Claim at a specific time (for testing).
    pub fn claim_at(&mut self, at: DateTime<Utc>) -> Result<ClaimReceipt> {
        if self.state == StreamState::Pending {
            return Err(Error::InvalidInput("stream not yet active".into()));
        }
        if self.state == StreamState::Closed {
            return Err(Error::InvalidInput("stream is closed".into()));
        }

        // Check claim interval
        if self.config.min_claim_interval_secs > 0
            && let Some(last) = self.last_claimed_at
        {
            let elapsed = (at - last).num_seconds().max(0) as u64;
            if elapsed < self.config.min_claim_interval_secs {
                return Err(Error::InvalidInput(format!(
                    "claim interval not met: {}s remaining",
                    self.config.min_claim_interval_secs - elapsed
                )));
            }
        }

        let amount = self.claimable(at);
        if amount == 0 {
            return Err(Error::InvalidInput("nothing to claim".into()));
        }

        self.total_claimed += amount;
        self.last_claimed_at = Some(at);

        let receipt = ClaimReceipt {
            stream_id: self.id.clone(),
            amount,
            claimed_at: at,
            elapsed_secs: self.total_active_seconds(at),
            remaining_deposit: self.remaining_deposit(at),
        };
        self.claims.push(receipt.clone());

        // Auto-close if depleted
        if self.remaining_deposit(at) == 0 && self.claimable(at) == 0 {
            self.state = StreamState::Closed;
        }

        Ok(receipt)
    }

    /// Cancel the stream. Unclaimed tokens return to the payer.
    /// Returns `(payee_gets, payer_refund)`.
    pub fn cancel(&mut self) -> Result<(u128, u128)> {
        self.cancel_at(Utc::now())
    }

    /// Cancel at a specific time (for testing).
    pub fn cancel_at(&mut self, at: DateTime<Utc>) -> Result<(u128, u128)> {
        if self.state == StreamState::Closed {
            return Err(Error::InvalidInput("stream already closed".into()));
        }
        let claimable = self.claimable(at);
        let payee_gets = self.total_claimed + claimable;
        let payer_refund = self.total_deposited.saturating_sub(payee_gets);

        self.total_claimed += claimable;
        self.state = StreamState::Closed;

        Ok((payee_gets, payer_refund))
    }

    /// Total number of claims made.
    pub fn claim_count(&self) -> usize {
        self.claims.len()
    }

    // ── Internal helpers ──

    fn active_seconds_since_last_resume(&self, now: DateTime<Utc>) -> u64 {
        let start = match self.started_at {
            Some(s) => s,
            None => return 0,
        };
        let elapsed = (now - start).num_seconds().max(0) as u64;
        // If max duration, cap it
        if self.config.max_duration_secs > 0 {
            let total = self.active_seconds_before_pause + elapsed;
            if total > self.config.max_duration_secs {
                return self
                    .config
                    .max_duration_secs
                    .saturating_sub(self.active_seconds_before_pause);
            }
        }
        elapsed
    }
}

/// Manages multiple concurrent payment streams.
#[derive(Debug, Default)]
pub struct StreamManager {
    streams: Vec<PaymentStream>,
}

impl StreamManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_stream(
        &mut self,
        payer: &str,
        payee: &str,
        config: StreamConfig,
        deposit: u128,
    ) -> Result<String> {
        let stream = PaymentStream::create(payer, payee, config, deposit)?;
        let id = stream.id.clone();
        self.streams.push(stream);
        Ok(id)
    }

    pub fn get(&self, id: &str) -> Option<&PaymentStream> {
        self.streams.iter().find(|s| s.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut PaymentStream> {
        self.streams.iter_mut().find(|s| s.id == id)
    }

    pub fn streams_for_payer(&self, payer: &str) -> Vec<&PaymentStream> {
        self.streams.iter().filter(|s| s.payer == payer).collect()
    }

    pub fn streams_for_payee(&self, payee: &str) -> Vec<&PaymentStream> {
        self.streams.iter().filter(|s| s.payee == payee).collect()
    }

    pub fn active_streams(&self) -> Vec<&PaymentStream> {
        self.streams
            .iter()
            .filter(|s| s.state == StreamState::Active)
            .collect()
    }

    pub fn total_outflow_rate(&self, payer: &str) -> u128 {
        self.streams
            .iter()
            .filter(|s| s.payer == payer && s.state == StreamState::Active)
            .map(|s| s.config.rate_per_second)
            .sum()
    }

    pub fn stream_count(&self) -> usize {
        self.streams.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn test_config() -> StreamConfig {
        StreamConfig {
            rate_per_second: 100,
            token: "USDC".into(),
            continuous_claim: true,
            min_claim_interval_secs: 0,
            max_duration_secs: 0,
        }
    }

    fn active_stream(start: DateTime<Utc>) -> PaymentStream {
        let mut s = PaymentStream::create("alice", "bob", test_config(), 10_000).unwrap();
        s.activate_at(start).unwrap();
        s
    }

    // ── Creation ───────────────────────────────────────────────

    #[test]
    fn create_stream() {
        let s = PaymentStream::create("alice", "bob", test_config(), 10_000).unwrap();
        assert_eq!(s.state, StreamState::Pending);
        assert_eq!(s.total_deposited, 10_000);
        assert!(s.id.starts_with("stream:"));
    }

    #[test]
    fn create_rejects_same_parties() {
        assert!(PaymentStream::create("alice", "alice", test_config(), 10_000).is_err());
    }

    #[test]
    fn create_rejects_zero_rate() {
        let mut cfg = test_config();
        cfg.rate_per_second = 0;
        assert!(PaymentStream::create("alice", "bob", cfg, 10_000).is_err());
    }

    #[test]
    fn create_rejects_zero_deposit() {
        assert!(PaymentStream::create("alice", "bob", test_config(), 0).is_err());
    }

    // ── Activation ─────────────────────────────────────────────

    #[test]
    fn activate_stream() {
        let mut s = PaymentStream::create("alice", "bob", test_config(), 10_000).unwrap();
        s.activate().unwrap();
        assert_eq!(s.state, StreamState::Active);
        assert!(s.started_at.is_some());
    }

    #[test]
    fn double_activate_fails() {
        let start = Utc::now();
        let mut s = active_stream(start);
        assert!(s.activate().is_err());
    }

    // ── Streaming math ─────────────────────────────────────────

    #[test]
    fn total_streamed_after_time() {
        let start = Utc::now();
        let s = active_stream(start);
        let after_10s = start + Duration::seconds(10);
        assert_eq!(s.total_streamed(after_10s), 1_000); // 100/s * 10s
    }

    #[test]
    fn total_streamed_capped_at_deposit() {
        let start = Utc::now();
        let s = active_stream(start); // 10,000 deposit at 100/s
        let after_200s = start + Duration::seconds(200); // would be 20,000
        assert_eq!(s.total_streamed(after_200s), 10_000);
    }

    #[test]
    fn claimable_matches_streamed() {
        let start = Utc::now();
        let s = active_stream(start);
        let after_5s = start + Duration::seconds(5);
        assert_eq!(s.claimable(after_5s), 500);
    }

    #[test]
    fn remaining_deposit_decreases() {
        let start = Utc::now();
        let s = active_stream(start);
        let after_30s = start + Duration::seconds(30);
        assert_eq!(s.remaining_deposit(after_30s), 7_000);
    }

    #[test]
    fn time_to_depletion() {
        let start = Utc::now();
        let s = active_stream(start); // 10,000 at 100/s = 100 seconds
        let ttd = s.time_to_depletion(start).unwrap();
        assert_eq!(ttd.num_seconds(), 100);
    }

    #[test]
    fn time_to_depletion_after_partial_stream() {
        let start = Utc::now();
        let s = active_stream(start);
        let after_50s = start + Duration::seconds(50);
        let ttd = s.time_to_depletion(after_50s).unwrap();
        assert_eq!(ttd.num_seconds(), 50);
    }

    // ── Claims ─────────────────────────────────────────────────

    #[test]
    fn claim_accumulated() {
        let start = Utc::now();
        let mut s = active_stream(start);
        let at = start + Duration::seconds(10);
        let receipt = s.claim_at(at).unwrap();
        assert_eq!(receipt.amount, 1_000);
        assert_eq!(s.total_claimed, 1_000);
        assert_eq!(s.claim_count(), 1);
    }

    #[test]
    fn claim_incremental() {
        let start = Utc::now();
        let mut s = active_stream(start);

        let r1 = s.claim_at(start + Duration::seconds(5)).unwrap();
        assert_eq!(r1.amount, 500);

        let r2 = s.claim_at(start + Duration::seconds(10)).unwrap();
        assert_eq!(r2.amount, 500); // only the new accumulation
        assert_eq!(s.total_claimed, 1_000);
    }

    #[test]
    fn claim_nothing_fails() {
        let start = Utc::now();
        let mut s = active_stream(start);
        assert!(s.claim_at(start).is_err());
    }

    #[test]
    fn claim_pending_fails() {
        let mut s = PaymentStream::create("alice", "bob", test_config(), 10_000).unwrap();
        assert!(s.claim().is_err());
    }

    #[test]
    fn claim_interval_enforced() {
        let mut cfg = test_config();
        cfg.min_claim_interval_secs = 60;
        let start = Utc::now();
        let mut s = PaymentStream::create("alice", "bob", cfg, 10_000).unwrap();
        s.activate_at(start).unwrap();

        s.claim_at(start + Duration::seconds(60)).unwrap();
        // Too soon for second claim
        assert!(s.claim_at(start + Duration::seconds(90)).is_err());
        // After interval
        assert!(s.claim_at(start + Duration::seconds(120)).is_ok());
    }

    #[test]
    fn auto_close_on_depletion() {
        let start = Utc::now();
        let mut s = active_stream(start);
        let at = start + Duration::seconds(100); // exactly depleted
        let receipt = s.claim_at(at).unwrap();
        assert_eq!(receipt.amount, 10_000);
        assert_eq!(s.state, StreamState::Closed);
    }

    // ── Pause / Resume ─────────────────────────────────────────

    #[test]
    fn pause_stops_streaming() {
        let start = Utc::now();
        let mut s = active_stream(start);

        s.pause_at(start + Duration::seconds(10)).unwrap();
        assert_eq!(s.state, StreamState::Paused);
        assert_eq!(s.active_seconds_before_pause, 10);

        // After pause, claimable stays the same regardless of time
        let way_later = start + Duration::seconds(1000);
        assert_eq!(s.claimable(way_later), 1_000);
    }

    #[test]
    fn resume_continues_streaming() {
        let start = Utc::now();
        let mut s = active_stream(start);

        s.pause_at(start + Duration::seconds(10)).unwrap(); // 10s active
        s.resume_at(start + Duration::seconds(20)).unwrap(); // 10s paused

        let at = start + Duration::seconds(30); // 10s more active
        assert_eq!(s.total_active_seconds(at), 20); // 10 + 10
        assert_eq!(s.claimable(at), 2_000);
    }

    #[test]
    fn pause_not_active_fails() {
        let mut s = PaymentStream::create("alice", "bob", test_config(), 10_000).unwrap();
        assert!(s.pause().is_err());
    }

    #[test]
    fn resume_not_paused_fails() {
        let start = Utc::now();
        let mut s = active_stream(start);
        assert!(s.resume().is_err());
    }

    // ── Top up ─────────────────────────────────────────────────

    #[test]
    fn top_up_extends_stream() {
        let start = Utc::now();
        let mut s = active_stream(start);
        s.top_up(5_000).unwrap();
        assert_eq!(s.total_deposited, 15_000);

        let ttd = s.time_to_depletion(start).unwrap();
        assert_eq!(ttd.num_seconds(), 150);
    }

    #[test]
    fn top_up_zero_fails() {
        let start = Utc::now();
        let mut s = active_stream(start);
        assert!(s.top_up(0).is_err());
    }

    #[test]
    fn top_up_closed_fails() {
        let start = Utc::now();
        let mut s = active_stream(start);
        s.cancel_at(start).unwrap();
        assert!(s.top_up(1_000).is_err());
    }

    // ── Cancel ─────────────────────────────────────────────────

    #[test]
    fn cancel_returns_split() {
        let start = Utc::now();
        let mut s = active_stream(start);
        let at = start + Duration::seconds(30); // 3,000 streamed
        let (payee_gets, payer_refund) = s.cancel_at(at).unwrap();

        assert_eq!(payee_gets, 3_000);
        assert_eq!(payer_refund, 7_000);
        assert_eq!(s.state, StreamState::Closed);
    }

    #[test]
    fn cancel_after_partial_claim() {
        let start = Utc::now();
        let mut s = active_stream(start);
        s.claim_at(start + Duration::seconds(10)).unwrap(); // claim 1,000
        let (payee_gets, payer_refund) = s.cancel_at(start + Duration::seconds(20)).unwrap();

        assert_eq!(payee_gets, 2_000); // 1,000 claimed + 1,000 unclaimed
        assert_eq!(payer_refund, 8_000);
    }

    #[test]
    fn cancel_closed_fails() {
        let start = Utc::now();
        let mut s = active_stream(start);
        s.cancel_at(start).unwrap();
        assert!(s.cancel().is_err());
    }

    // ── Max duration ───────────────────────────────────────────

    #[test]
    fn max_duration_caps_streaming() {
        let mut cfg = test_config();
        cfg.max_duration_secs = 50;
        let start = Utc::now();
        let mut s = PaymentStream::create("alice", "bob", cfg, 10_000).unwrap();
        s.activate_at(start).unwrap();

        let way_later = start + Duration::seconds(200);
        // Capped at 50s * 100/s = 5,000 (not 20,000)
        assert_eq!(s.total_streamed(way_later), 5_000);
        assert_eq!(s.total_active_seconds(way_later), 50);
    }

    // ── StreamManager ──────────────────────────────────────────

    #[test]
    fn manager_create_and_get() {
        let mut mgr = StreamManager::new();
        let id = mgr
            .create_stream("alice", "bob", test_config(), 10_000)
            .unwrap();
        assert!(mgr.get(&id).is_some());
        assert_eq!(mgr.stream_count(), 1);
    }

    #[test]
    fn manager_filter_by_payer() {
        let mut mgr = StreamManager::new();
        mgr.create_stream("alice", "bob", test_config(), 1_000)
            .unwrap();
        mgr.create_stream("alice", "carol", test_config(), 2_000)
            .unwrap();
        mgr.create_stream("bob", "carol", test_config(), 3_000)
            .unwrap();

        assert_eq!(mgr.streams_for_payer("alice").len(), 2);
        assert_eq!(mgr.streams_for_payer("bob").len(), 1);
    }

    #[test]
    fn manager_total_outflow() {
        let mut mgr = StreamManager::new();
        let id1 = mgr
            .create_stream("alice", "bob", test_config(), 1_000)
            .unwrap();
        let id2 = mgr
            .create_stream("alice", "carol", test_config(), 2_000)
            .unwrap();

        mgr.get_mut(&id1).unwrap().activate().unwrap();
        mgr.get_mut(&id2).unwrap().activate().unwrap();

        assert_eq!(mgr.total_outflow_rate("alice"), 200); // 100 + 100
    }

    #[test]
    fn manager_active_streams() {
        let mut mgr = StreamManager::new();
        let id1 = mgr
            .create_stream("alice", "bob", test_config(), 1_000)
            .unwrap();
        mgr.create_stream("alice", "carol", test_config(), 2_000)
            .unwrap();

        mgr.get_mut(&id1).unwrap().activate().unwrap();

        assert_eq!(mgr.active_streams().len(), 1);
    }

    // ── Serialization ──────────────────────────────────────────

    #[test]
    fn stream_serializes() {
        let start = Utc::now();
        let s = active_stream(start);
        let json = serde_json::to_string(&s).unwrap();
        let restored: PaymentStream = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.payer, "alice");
        assert_eq!(restored.config.rate_per_second, 100);
    }

    #[test]
    fn claim_receipt_serializes() {
        let start = Utc::now();
        let mut s = active_stream(start);
        let receipt = s.claim_at(start + Duration::seconds(5)).unwrap();
        let json = serde_json::to_string(&receipt).unwrap();
        let restored: ClaimReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.amount, 500);
    }
}
