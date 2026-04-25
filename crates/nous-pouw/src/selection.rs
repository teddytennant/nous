//! Deterministic worker selection.
//!
//! Given the previous block's hash and an epoch counter, every node derives
//! the same set of selected workers — no leader, no oracle. Workers are
//! ranked by a VRF-style score; the top `n` (weighted toward higher
//! `stake * trust`) are picked.
//!
//! This is *not* a real VRF (no proof-of-output) — for v0 the deterministic
//! hash is enough because every honest node can recompute it. v1 can swap
//! in [VRF-RFC9381](https://datatracker.ietf.org/doc/html/rfc9381) without
//! changing call sites.

use crate::state::{ChainState, WorkerId};

/// Score one worker for one (prev_hash, epoch) tuple.
///
/// Lower score = higher priority (so we sort ascending and take the first n).
pub fn vrf_score(prev_hash: &[u8; 32], epoch: u64, worker: &WorkerId) -> u64 {
    let mut h = blake3::Hasher::new();
    h.update(prev_hash);
    h.update(&epoch.to_le_bytes());
    h.update(&worker.0);
    let bytes = h.finalize();
    let raw: [u8; 8] = bytes.as_bytes()[..8].try_into().unwrap();
    u64::from_le_bytes(raw)
}

/// Select up to `n` workers for a job at this epoch.
///
/// We bias the selection by weight: each worker's effective score is
/// `vrf_score / max(weight, 1.0)`, so heavier workers have more chances of
/// landing near the top while light workers still appear occasionally
/// (preserves liveness when most stake is concentrated).
pub fn select_workers(
    state: &ChainState,
    prev_hash: &[u8; 32],
    epoch: u64,
    n: usize,
) -> Vec<WorkerId> {
    let mut scored: Vec<(f64, WorkerId)> = state
        .eligible_workers()
        .into_iter()
        .map(|w| {
            let raw = vrf_score(prev_hash, epoch, &w) as f64;
            let weight = state
                .workers
                .get(&w)
                .map(|i| i.weight().max(1.0))
                .unwrap_or(1.0);
            (raw / weight, w)
        })
        .collect();
    // Sort by score ascending; ties broken by WorkerId for determinism.
    scored.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.cmp(&b.1))
    });
    scored.into_iter().take(n).map(|(_, w)| w).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn make_state(n_workers: usize, stake: u64, trust: f64) -> ChainState {
        let mut s = ChainState::new();
        for _ in 0..n_workers {
            let sk = SigningKey::generate(&mut OsRng);
            s.register_worker(
                WorkerId::from_verifying_key(&sk.verifying_key()),
                stake,
                trust,
            );
        }
        s
    }

    #[test]
    fn selection_is_deterministic() {
        let state = make_state(20, 100, 1.0);
        let a = select_workers(&state, &[42u8; 32], 7, 5);
        let b = select_workers(&state, &[42u8; 32], 7, 5);
        assert_eq!(a, b);
    }

    #[test]
    fn selection_changes_with_prev_hash() {
        let state = make_state(20, 100, 1.0);
        let a = select_workers(&state, &[1u8; 32], 0, 5);
        let b = select_workers(&state, &[2u8; 32], 0, 5);
        assert_ne!(a, b);
    }

    #[test]
    fn selection_changes_with_epoch() {
        let state = make_state(20, 100, 1.0);
        let a = select_workers(&state, &[1u8; 32], 0, 5);
        let b = select_workers(&state, &[1u8; 32], 1, 5);
        assert_ne!(a, b);
    }

    #[test]
    fn selection_returns_at_most_n() {
        let state = make_state(3, 100, 1.0);
        let s = select_workers(&state, &[0u8; 32], 0, 5);
        assert_eq!(s.len(), 3); // only 3 eligible
    }

    #[test]
    fn selection_excludes_slashed() {
        let mut state = make_state(5, 100, 1.0);
        // slash the first worker
        let first = *state.workers.keys().next().unwrap();
        state.workers.get_mut(&first).unwrap().slashed = true;
        let s = select_workers(&state, &[0u8; 32], 0, 10);
        assert!(!s.contains(&first));
    }

    #[test]
    fn empty_state_returns_empty_selection() {
        let state = ChainState::new();
        assert!(select_workers(&state, &[0u8; 32], 0, 5).is_empty());
    }

    #[test]
    fn vrf_score_changes_per_input() {
        let w = WorkerId([7u8; 32]);
        assert_ne!(vrf_score(&[0u8; 32], 0, &w), vrf_score(&[0u8; 32], 1, &w));
        assert_ne!(vrf_score(&[0u8; 32], 0, &w), vrf_score(&[1u8; 32], 0, &w));
    }

    #[test]
    fn heavy_workers_appear_more_often() {
        // Statistical test: across many epochs, heavy worker should land in
        // the selection more than half the time.
        let mut state = ChainState::new();
        let heavy_sk = SigningKey::generate(&mut OsRng);
        let heavy = WorkerId::from_verifying_key(&heavy_sk.verifying_key());
        state.register_worker(heavy, 10_000, 1.0);
        for _ in 0..10 {
            let sk = SigningKey::generate(&mut OsRng);
            state.register_worker(WorkerId::from_verifying_key(&sk.verifying_key()), 10, 0.1);
        }
        let mut hits = 0;
        for epoch in 0..200 {
            let s = select_workers(&state, &[epoch as u8; 32], epoch, 1);
            if s.first() == Some(&heavy) {
                hits += 1;
            }
        }
        // 10000 vs sum(10*0.1)*10 = 10 weight; heavy should dominate strongly.
        assert!(hits > 100, "heavy worker hit only {hits}/200");
    }
}
