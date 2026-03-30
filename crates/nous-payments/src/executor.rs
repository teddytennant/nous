//! Atomic swap execution engine with HTLC lifecycle management.
//!
//! Manages the full lifecycle of hash-time-locked contract (HTLC) swaps:
//! 1. **Initiate** — Creator publishes an order with a hashlock
//! 2. **Match** — Counterparty accepts and locks their funds
//! 3. **Claim** — Creator reveals preimage, counterparty claims
//! 4. **Refund** — If timeout expires, parties reclaim their funds
//!
//! The executor tracks state transitions, enforces timeouts, and produces
//! an audit trail of all swap operations.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::swap::{SwapOrder, SwapStatus};

/// Phase of the HTLC swap from the executor's perspective.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwapPhase {
    /// Order published, waiting for counterparty.
    Published,
    /// Both sides locked, waiting for preimage reveal.
    Locked,
    /// Preimage revealed, waiting for claim.
    Revealed,
    /// Swap completed — both sides claimed.
    Settled,
    /// Swap refunded — timeout expired.
    Refunded,
    /// Swap cancelled before match.
    Cancelled,
}

/// A tracked swap in the executor with full audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedSwap {
    pub id: String,
    pub order: SwapOrder,
    pub phase: SwapPhase,
    pub hashlock: Vec<u8>,
    pub preimage: Option<Vec<u8>>,
    pub initiated_at: DateTime<Utc>,
    pub locked_at: Option<DateTime<Utc>>,
    pub settled_at: Option<DateTime<Utc>>,
    pub events: Vec<SwapEvent>,
}

/// An event in the swap's lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapEvent {
    pub timestamp: DateTime<Utc>,
    pub action: SwapAction,
    pub actor: String,
}

/// Actions that can occur during a swap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwapAction {
    Published,
    Matched { counterparty: String },
    PreimageRevealed,
    Claimed { by: String },
    Refunded { by: String },
    Cancelled,
    Expired,
}

/// Statistics about the swap executor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorStats {
    pub total_swaps: usize,
    pub active: usize,
    pub settled: usize,
    pub refunded: usize,
    pub cancelled: usize,
    pub total_volume: HashMap<String, u128>,
}

/// The swap execution engine.
pub struct SwapExecutor {
    swaps: HashMap<String, TrackedSwap>,
    /// Index: initiator DID → swap IDs.
    by_initiator: HashMap<String, Vec<String>>,
    /// Index: token pair → swap IDs (for matching).
    by_pair: HashMap<(String, String), Vec<String>>,
}

impl SwapExecutor {
    pub fn new() -> Self {
        Self {
            swaps: HashMap::new(),
            by_initiator: HashMap::new(),
            by_pair: HashMap::new(),
        }
    }

    /// Initiate a new swap. Generates a hashlock from a random preimage.
    /// Returns the swap ID and the secret preimage (caller must store this).
    pub fn initiate(&mut self, order: SwapOrder) -> Result<(String, Vec<u8>), String> {
        if order.status != SwapStatus::Pending {
            return Err("order must be in Pending status".into());
        }

        // Generate preimage and hashlock.
        let preimage: Vec<u8> = (0..32)
            .map(|i| {
                // Deterministic-ish from order ID for testability, but in production
                // this would use a CSPRNG.
                let mut hasher = Sha256::new();
                hasher.update(order.id.as_bytes());
                hasher.update([i as u8]);
                hasher.update(Utc::now().timestamp_nanos_opt().unwrap_or(0).to_le_bytes());
                hasher.finalize()[0]
            })
            .collect();

        let hashlock = sha256(&preimage);
        let swap_id = format!("swap:{}", Uuid::new_v4());

        let tracked = TrackedSwap {
            id: swap_id.clone(),
            order: order.clone(),
            phase: SwapPhase::Published,
            hashlock: hashlock.clone(),
            preimage: None, // Only stored when revealed.
            initiated_at: Utc::now(),
            locked_at: None,
            settled_at: None,
            events: vec![SwapEvent {
                timestamp: Utc::now(),
                action: SwapAction::Published,
                actor: order.initiator.clone(),
            }],
        };

        // Update indexes.
        self.by_initiator
            .entry(order.initiator.clone())
            .or_default()
            .push(swap_id.clone());
        self.by_pair
            .entry((order.offer_token.clone(), order.want_token.clone()))
            .or_default()
            .push(swap_id.clone());

        self.swaps.insert(swap_id.clone(), tracked);
        Ok((swap_id, preimage))
    }

    /// Match a published swap. The counterparty locks their side.
    pub fn match_swap(&mut self, swap_id: &str, counterparty: &str) -> Result<(), String> {
        let swap = self.swaps.get_mut(swap_id).ok_or("swap not found")?;

        if swap.phase != SwapPhase::Published {
            return Err(format!(
                "swap is in {:?} phase, expected Published",
                swap.phase
            ));
        }

        if swap.order.is_expired() {
            swap.phase = SwapPhase::Cancelled;
            swap.events.push(SwapEvent {
                timestamp: Utc::now(),
                action: SwapAction::Expired,
                actor: counterparty.to_string(),
            });
            return Err("swap has expired".into());
        }

        if counterparty == swap.order.initiator {
            return Err("cannot match your own swap".into());
        }

        swap.phase = SwapPhase::Locked;
        swap.locked_at = Some(Utc::now());
        swap.events.push(SwapEvent {
            timestamp: Utc::now(),
            action: SwapAction::Matched {
                counterparty: counterparty.to_string(),
            },
            actor: counterparty.to_string(),
        });

        Ok(())
    }

    /// Reveal the preimage to unlock the swap.
    /// Only the initiator should call this.
    pub fn reveal_preimage(&mut self, swap_id: &str, preimage: &[u8]) -> Result<(), String> {
        let swap = self.swaps.get_mut(swap_id).ok_or("swap not found")?;

        if swap.phase != SwapPhase::Locked {
            return Err(format!(
                "swap is in {:?} phase, expected Locked",
                swap.phase
            ));
        }

        // Verify preimage matches hashlock.
        let hash = sha256(preimage);
        if hash != swap.hashlock {
            return Err("preimage does not match hashlock".into());
        }

        swap.preimage = Some(preimage.to_vec());
        swap.phase = SwapPhase::Revealed;
        swap.events.push(SwapEvent {
            timestamp: Utc::now(),
            action: SwapAction::PreimageRevealed,
            actor: swap.order.initiator.clone(),
        });

        Ok(())
    }

    /// Settle the swap — both parties claim their funds.
    /// In a real system, this would execute on-chain transfers.
    pub fn settle(&mut self, swap_id: &str, claimer: &str) -> Result<(), String> {
        let swap = self.swaps.get_mut(swap_id).ok_or("swap not found")?;

        if swap.phase != SwapPhase::Revealed {
            return Err(format!(
                "swap is in {:?} phase, expected Revealed",
                swap.phase
            ));
        }

        swap.phase = SwapPhase::Settled;
        swap.settled_at = Some(Utc::now());
        swap.events.push(SwapEvent {
            timestamp: Utc::now(),
            action: SwapAction::Claimed {
                by: claimer.to_string(),
            },
            actor: claimer.to_string(),
        });

        Ok(())
    }

    /// Refund a swap that has expired.
    pub fn refund(&mut self, swap_id: &str, caller: &str) -> Result<(), String> {
        let swap = self.swaps.get_mut(swap_id).ok_or("swap not found")?;

        match swap.phase {
            SwapPhase::Published | SwapPhase::Locked => {}
            _ => {
                return Err(format!("cannot refund swap in {:?} phase", swap.phase));
            }
        }

        if !swap.order.is_expired() && swap.phase == SwapPhase::Locked {
            return Err("cannot refund locked swap before expiry".into());
        }

        if caller != swap.order.initiator {
            return Err("only initiator can refund".into());
        }

        swap.phase = SwapPhase::Refunded;
        swap.events.push(SwapEvent {
            timestamp: Utc::now(),
            action: SwapAction::Refunded {
                by: caller.to_string(),
            },
            actor: caller.to_string(),
        });

        Ok(())
    }

    /// Cancel an unpublished swap.
    pub fn cancel(&mut self, swap_id: &str, caller: &str) -> Result<(), String> {
        let swap = self.swaps.get_mut(swap_id).ok_or("swap not found")?;

        if swap.phase != SwapPhase::Published {
            return Err("can only cancel published swaps".into());
        }

        if caller != swap.order.initiator {
            return Err("only initiator can cancel".into());
        }

        swap.phase = SwapPhase::Cancelled;
        swap.events.push(SwapEvent {
            timestamp: Utc::now(),
            action: SwapAction::Cancelled,
            actor: caller.to_string(),
        });

        Ok(())
    }

    /// Get a swap by ID.
    pub fn get(&self, swap_id: &str) -> Option<&TrackedSwap> {
        self.swaps.get(swap_id)
    }

    /// Get all swaps by initiator.
    pub fn by_initiator(&self, initiator: &str) -> Vec<&TrackedSwap> {
        self.by_initiator
            .get(initiator)
            .map(|ids| ids.iter().filter_map(|id| self.swaps.get(id)).collect())
            .unwrap_or_default()
    }

    /// Find matching swaps for a given token pair.
    pub fn find_matches(&self, want_token: &str, offer_token: &str) -> Vec<&TrackedSwap> {
        // Look for swaps offering what we want, wanting what we offer.
        self.by_pair
            .get(&(want_token.to_string(), offer_token.to_string()))
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.swaps.get(id))
                    .filter(|s| s.phase == SwapPhase::Published)
                    .filter(|s| !s.order.is_expired())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get executor statistics.
    pub fn stats(&self) -> ExecutorStats {
        let mut active = 0;
        let mut settled = 0;
        let mut refunded = 0;
        let mut cancelled = 0;
        let mut volume: HashMap<String, u128> = HashMap::new();

        for swap in self.swaps.values() {
            match swap.phase {
                SwapPhase::Published | SwapPhase::Locked | SwapPhase::Revealed => active += 1,
                SwapPhase::Settled => {
                    settled += 1;
                    *volume.entry(swap.order.offer_token.clone()).or_insert(0) +=
                        swap.order.offer_amount;
                    *volume.entry(swap.order.want_token.clone()).or_insert(0) +=
                        swap.order.want_amount;
                }
                SwapPhase::Refunded => refunded += 1,
                SwapPhase::Cancelled => cancelled += 1,
            }
        }

        ExecutorStats {
            total_swaps: self.swaps.len(),
            active,
            settled,
            refunded,
            cancelled,
            total_volume: volume,
        }
    }

    /// Prune expired swaps (mark them as cancelled).
    pub fn prune_expired(&mut self) -> usize {
        let expired_ids: Vec<String> = self
            .swaps
            .iter()
            .filter(|(_, s)| s.order.is_expired() && s.phase == SwapPhase::Published)
            .map(|(id, _)| id.clone())
            .collect();

        let count = expired_ids.len();
        for id in &expired_ids {
            if let Some(swap) = self.swaps.get_mut(id) {
                swap.phase = SwapPhase::Cancelled;
                swap.events.push(SwapEvent {
                    timestamp: Utc::now(),
                    action: SwapAction::Expired,
                    actor: "system".to_string(),
                });
            }
        }
        count
    }
}

impl Default for SwapExecutor {
    fn default() -> Self {
        Self::new()
    }
}

fn sha256(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_order(initiator: &str) -> SwapOrder {
        SwapOrder::new(
            initiator,
            "ETH",
            1_000_000_000_000_000_000, // 1 ETH
            "USDC",
            2_000_000_000, // 2000 USDC
            24,            // 24 hours TTL
        )
        .unwrap()
    }

    #[test]
    fn empty_executor() {
        let exec = SwapExecutor::new();
        let stats = exec.stats();
        assert_eq!(stats.total_swaps, 0);
    }

    #[test]
    fn initiate_swap() {
        let mut exec = SwapExecutor::new();
        let order = make_order("did:key:alice");
        let (swap_id, preimage) = exec.initiate(order).unwrap();

        assert!(swap_id.starts_with("swap:"));
        assert_eq!(preimage.len(), 32);

        let swap = exec.get(&swap_id).unwrap();
        assert_eq!(swap.phase, SwapPhase::Published);
        assert_eq!(swap.events.len(), 1);
    }

    #[test]
    fn match_swap() {
        let mut exec = SwapExecutor::new();
        let order = make_order("did:key:alice");
        let (swap_id, _) = exec.initiate(order).unwrap();

        exec.match_swap(&swap_id, "did:key:bob").unwrap();

        let swap = exec.get(&swap_id).unwrap();
        assert_eq!(swap.phase, SwapPhase::Locked);
        assert!(swap.locked_at.is_some());
    }

    #[test]
    fn cannot_match_own_swap() {
        let mut exec = SwapExecutor::new();
        let order = make_order("did:key:alice");
        let (swap_id, _) = exec.initiate(order).unwrap();

        let err = exec.match_swap(&swap_id, "did:key:alice").unwrap_err();
        assert!(err.contains("cannot match your own swap"));
    }

    #[test]
    fn reveal_preimage() {
        let mut exec = SwapExecutor::new();
        let order = make_order("did:key:alice");
        let (swap_id, preimage) = exec.initiate(order).unwrap();
        exec.match_swap(&swap_id, "did:key:bob").unwrap();

        exec.reveal_preimage(&swap_id, &preimage).unwrap();

        let swap = exec.get(&swap_id).unwrap();
        assert_eq!(swap.phase, SwapPhase::Revealed);
        assert!(swap.preimage.is_some());
    }

    #[test]
    fn reveal_wrong_preimage() {
        let mut exec = SwapExecutor::new();
        let order = make_order("did:key:alice");
        let (swap_id, _) = exec.initiate(order).unwrap();
        exec.match_swap(&swap_id, "did:key:bob").unwrap();

        let err = exec
            .reveal_preimage(&swap_id, b"wrong-preimage")
            .unwrap_err();
        assert!(err.contains("does not match"));
    }

    #[test]
    fn settle_swap() {
        let mut exec = SwapExecutor::new();
        let order = make_order("did:key:alice");
        let (swap_id, preimage) = exec.initiate(order).unwrap();
        exec.match_swap(&swap_id, "did:key:bob").unwrap();
        exec.reveal_preimage(&swap_id, &preimage).unwrap();

        exec.settle(&swap_id, "did:key:bob").unwrap();

        let swap = exec.get(&swap_id).unwrap();
        assert_eq!(swap.phase, SwapPhase::Settled);
        assert!(swap.settled_at.is_some());
    }

    #[test]
    fn full_lifecycle() {
        let mut exec = SwapExecutor::new();
        let order = make_order("did:key:alice");
        let (swap_id, preimage) = exec.initiate(order).unwrap();
        exec.match_swap(&swap_id, "did:key:bob").unwrap();
        exec.reveal_preimage(&swap_id, &preimage).unwrap();
        exec.settle(&swap_id, "did:key:bob").unwrap();

        let swap = exec.get(&swap_id).unwrap();
        assert_eq!(swap.events.len(), 4);
        assert_eq!(swap.phase, SwapPhase::Settled);
    }

    #[test]
    fn cancel_published_swap() {
        let mut exec = SwapExecutor::new();
        let order = make_order("did:key:alice");
        let (swap_id, _) = exec.initiate(order).unwrap();

        exec.cancel(&swap_id, "did:key:alice").unwrap();

        let swap = exec.get(&swap_id).unwrap();
        assert_eq!(swap.phase, SwapPhase::Cancelled);
    }

    #[test]
    fn cannot_cancel_locked_swap() {
        let mut exec = SwapExecutor::new();
        let order = make_order("did:key:alice");
        let (swap_id, _) = exec.initiate(order).unwrap();
        exec.match_swap(&swap_id, "did:key:bob").unwrap();

        let err = exec.cancel(&swap_id, "did:key:alice").unwrap_err();
        assert!(err.contains("can only cancel published"));
    }

    #[test]
    fn only_initiator_can_cancel() {
        let mut exec = SwapExecutor::new();
        let order = make_order("did:key:alice");
        let (swap_id, _) = exec.initiate(order).unwrap();

        let err = exec.cancel(&swap_id, "did:key:bob").unwrap_err();
        assert!(err.contains("only initiator"));
    }

    #[test]
    fn refund_published_swap() {
        let mut exec = SwapExecutor::new();
        let order = make_order("did:key:alice");
        let (swap_id, _) = exec.initiate(order).unwrap();

        exec.refund(&swap_id, "did:key:alice").unwrap();

        let swap = exec.get(&swap_id).unwrap();
        assert_eq!(swap.phase, SwapPhase::Refunded);
    }

    #[test]
    fn cannot_settle_without_reveal() {
        let mut exec = SwapExecutor::new();
        let order = make_order("did:key:alice");
        let (swap_id, _) = exec.initiate(order).unwrap();
        exec.match_swap(&swap_id, "did:key:bob").unwrap();

        let err = exec.settle(&swap_id, "did:key:bob").unwrap_err();
        assert!(err.contains("expected Revealed"));
    }

    #[test]
    fn by_initiator() {
        let mut exec = SwapExecutor::new();

        let order1 = make_order("did:key:alice");
        let order2 = make_order("did:key:alice");
        let order3 = make_order("did:key:bob");

        exec.initiate(order1).unwrap();
        exec.initiate(order2).unwrap();
        exec.initiate(order3).unwrap();

        assert_eq!(exec.by_initiator("did:key:alice").len(), 2);
        assert_eq!(exec.by_initiator("did:key:bob").len(), 1);
        assert_eq!(exec.by_initiator("did:key:carol").len(), 0);
    }

    #[test]
    fn find_matches() {
        let mut exec = SwapExecutor::new();
        let order = make_order("did:key:alice"); // offers ETH, wants USDC

        exec.initiate(order).unwrap();

        // Search for swaps offering ETH wanting USDC.
        let matches = exec.find_matches("ETH", "USDC");
        assert_eq!(matches.len(), 1);

        // No match for inverse pair.
        let matches = exec.find_matches("USDC", "ETH");
        assert!(matches.is_empty());
    }

    #[test]
    fn stats_tracking() {
        let mut exec = SwapExecutor::new();
        let order = make_order("did:key:alice");
        let (swap_id, preimage) = exec.initiate(order).unwrap();
        exec.match_swap(&swap_id, "did:key:bob").unwrap();
        exec.reveal_preimage(&swap_id, &preimage).unwrap();
        exec.settle(&swap_id, "did:key:bob").unwrap();

        let stats = exec.stats();
        assert_eq!(stats.total_swaps, 1);
        assert_eq!(stats.settled, 1);
        assert_eq!(stats.active, 0);
        assert!(stats.total_volume.contains_key("ETH"));
        assert!(stats.total_volume.contains_key("USDC"));
    }

    #[test]
    fn swap_not_found() {
        let mut exec = SwapExecutor::new();
        assert!(exec.match_swap("nonexistent", "did:key:bob").is_err());
        assert!(exec.reveal_preimage("nonexistent", b"test").is_err());
        assert!(exec.settle("nonexistent", "did:key:bob").is_err());
    }

    #[test]
    fn tracked_swap_serializes() {
        let mut exec = SwapExecutor::new();
        let order = make_order("did:key:alice");
        let (swap_id, _) = exec.initiate(order).unwrap();

        let swap = exec.get(&swap_id).unwrap();
        let json = serde_json::to_string(swap).unwrap();
        let restored: TrackedSwap = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, swap.id);
        assert_eq!(restored.phase, SwapPhase::Published);
    }

    #[test]
    fn executor_stats_serializes() {
        let exec = SwapExecutor::new();
        let stats = exec.stats();
        let json = serde_json::to_string(&stats).unwrap();
        let restored: ExecutorStats = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.total_swaps, 0);
    }
}
