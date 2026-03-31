//! Sybil resistance scoring for governance participation.
//!
//! Evaluates identity trustworthiness across multiple dimensions to
//! produce a composite score. Governance operations (voting, proposing,
//! delegating) can require a minimum sybil score to participate.
//!
//! Scoring dimensions:
//! - **Identity age**: How long the DID has existed
//! - **Social vouches**: Number of unique vouches from other scored identities
//! - **On-chain activity**: Transaction and governance participation history
//! - **Stake amount**: Tokens staked as skin-in-the-game
//! - **Unique device binding**: Whether the identity is bound to a unique device
//! - **Credential count**: Number of verifiable credentials attached

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A trust factor contributing to the sybil score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrustFactor {
    /// How long the identity has existed (in days).
    IdentityAge,
    /// Number of unique social vouches from other scored identities.
    SocialVouches,
    /// Number of on-chain transactions and governance actions.
    OnChainActivity,
    /// Amount of tokens staked.
    StakeAmount,
    /// Whether the identity is bound to a unique device attestation.
    DeviceBinding,
    /// Number of verifiable credentials held.
    CredentialCount,
}

/// Raw evidence for a single trust factor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustEvidence {
    pub factor: TrustFactor,
    /// Raw numeric value (days for age, count for vouches, tokens for stake, etc.).
    pub raw_value: f64,
}

/// A computed sybil resistance score for an identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SybilScore {
    /// The composite score, 0.0 (untrusted) to 1.0 (fully trusted).
    pub score: f64,
    /// Individual factor scores (0.0–1.0 each).
    pub factors: HashMap<TrustFactor, f64>,
    /// The DID this score belongs to.
    pub did: String,
    /// Whether the identity passes the given threshold.
    pub eligible: bool,
    /// The threshold used for eligibility.
    pub threshold: f64,
}

/// Weight configuration for each trust factor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScorerWeights {
    weights: HashMap<TrustFactor, f64>,
}

impl ScorerWeights {
    /// Create weights with sensible defaults.
    ///
    /// Default distribution (sums to 1.0):
    /// - Identity age: 0.15
    /// - Social vouches: 0.25
    /// - On-chain activity: 0.20
    /// - Stake amount: 0.15
    /// - Device binding: 0.15
    /// - Credential count: 0.10
    pub fn default_weights() -> Self {
        let mut weights = HashMap::new();
        weights.insert(TrustFactor::IdentityAge, 0.15);
        weights.insert(TrustFactor::SocialVouches, 0.25);
        weights.insert(TrustFactor::OnChainActivity, 0.20);
        weights.insert(TrustFactor::StakeAmount, 0.15);
        weights.insert(TrustFactor::DeviceBinding, 0.15);
        weights.insert(TrustFactor::CredentialCount, 0.10);
        Self { weights }
    }

    /// Create custom weights. They will be normalized to sum to 1.0.
    pub fn custom(weights: HashMap<TrustFactor, f64>) -> Self {
        Self { weights }
    }

    /// Get the weight for a factor, defaulting to 0 if not configured.
    pub fn get(&self, factor: &TrustFactor) -> f64 {
        self.weights.get(factor).copied().unwrap_or(0.0)
    }

    /// Total weight sum (for normalization).
    pub fn total(&self) -> f64 {
        self.weights.values().sum()
    }

    /// Get the normalized weight (weight / total).
    pub fn normalized(&self, factor: &TrustFactor) -> f64 {
        let total = self.total();
        if total == 0.0 {
            return 0.0;
        }
        self.get(factor) / total
    }
}

impl Default for ScorerWeights {
    fn default() -> Self {
        Self::default_weights()
    }
}

/// Configuration for how raw values map to 0.0–1.0 factor scores.
#[derive(Debug, Clone)]
pub struct NormalizationConfig {
    /// Identity age: days needed for maximum score.
    pub age_max_days: f64,
    /// Social vouches: count needed for maximum score.
    pub vouches_max: f64,
    /// On-chain activity: action count for maximum score.
    pub activity_max: f64,
    /// Stake: token amount for maximum score.
    pub stake_max: f64,
    /// Credentials: count for maximum score.
    pub credentials_max: f64,
}

impl Default for NormalizationConfig {
    fn default() -> Self {
        Self {
            age_max_days: 365.0,  // 1 year for full age score
            vouches_max: 10.0,    // 10 vouches for full vouch score
            activity_max: 100.0,  // 100 actions for full activity score
            stake_max: 10_000.0,  // 10k tokens for full stake score
            credentials_max: 5.0, // 5 credentials for full credential score
        }
    }
}

/// Sybil resistance scorer.
///
/// Evaluates trust evidence against configurable weights and normalization
/// curves to produce a composite score.
#[derive(Debug, Clone)]
pub struct SybilScorer {
    weights: ScorerWeights,
    normalization: NormalizationConfig,
    /// Default eligibility threshold (0.0–1.0).
    threshold: f64,
}

impl SybilScorer {
    /// Create a scorer with default configuration.
    pub fn new() -> Self {
        Self {
            weights: ScorerWeights::default(),
            normalization: NormalizationConfig::default(),
            threshold: 0.3,
        }
    }

    /// Create a scorer with custom weights.
    pub fn with_weights(weights: ScorerWeights) -> Self {
        Self {
            weights,
            ..Self::new()
        }
    }

    /// Set the eligibility threshold.
    pub fn set_threshold(&mut self, threshold: f64) {
        self.threshold = threshold.clamp(0.0, 1.0);
    }

    /// Get the current threshold.
    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Set custom normalization configuration.
    pub fn set_normalization(&mut self, config: NormalizationConfig) {
        self.normalization = config;
    }

    /// Score an identity based on provided trust evidence.
    pub fn score(&self, did: &str, evidence: &[TrustEvidence]) -> SybilScore {
        let mut factors = HashMap::new();

        for ev in evidence {
            let normalized = self.normalize_factor(&ev.factor, ev.raw_value);
            factors.insert(ev.factor, normalized);
        }

        // Compute weighted sum
        let mut composite = 0.0;
        let total_weight = self.weights.total();

        if total_weight > 0.0 {
            for (factor, &factor_score) in &factors {
                let weight = self.weights.normalized(factor);
                composite += factor_score * weight;
            }
        }

        // Clamp to [0, 1]
        composite = composite.clamp(0.0, 1.0);

        SybilScore {
            score: composite,
            factors,
            did: did.to_string(),
            eligible: composite >= self.threshold,
            threshold: self.threshold,
        }
    }

    /// Check if an identity meets the eligibility threshold.
    pub fn is_eligible(&self, did: &str, evidence: &[TrustEvidence]) -> bool {
        self.score(did, evidence).eligible
    }

    /// Score multiple identities at once.
    pub fn score_batch(
        &self,
        identities: &HashMap<String, Vec<TrustEvidence>>,
    ) -> HashMap<String, SybilScore> {
        identities
            .iter()
            .map(|(did, evidence)| (did.clone(), self.score(did, evidence)))
            .collect()
    }

    /// Filter a set of identities to only those that are eligible.
    pub fn filter_eligible(&self, identities: &HashMap<String, Vec<TrustEvidence>>) -> Vec<String> {
        identities
            .iter()
            .filter(|(did, evidence)| self.is_eligible(did, evidence))
            .map(|(did, _)| did.clone())
            .collect()
    }

    /// Normalize a raw factor value to 0.0–1.0.
    ///
    /// Uses a diminishing returns curve: `min(raw / max, 1.0)^0.5`
    /// This means getting halfway to max gives you ~71% of the score,
    /// making early contributions more impactful.
    fn normalize_factor(&self, factor: &TrustFactor, raw: f64) -> f64 {
        if raw <= 0.0 {
            return 0.0;
        }

        let max = match factor {
            TrustFactor::IdentityAge => self.normalization.age_max_days,
            TrustFactor::SocialVouches => self.normalization.vouches_max,
            TrustFactor::OnChainActivity => self.normalization.activity_max,
            TrustFactor::StakeAmount => self.normalization.stake_max,
            TrustFactor::CredentialCount => self.normalization.credentials_max,
            TrustFactor::DeviceBinding => 1.0, // Binary: 0 or 1
        };

        if max <= 0.0 {
            return 0.0;
        }

        // Device binding is binary
        if *factor == TrustFactor::DeviceBinding {
            return if raw >= 1.0 { 1.0 } else { 0.0 };
        }

        // Diminishing returns: sqrt of linear ratio
        let linear = (raw / max).min(1.0);
        linear.sqrt()
    }
}

impl Default for SybilScorer {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to create trust evidence.
pub fn evidence(factor: TrustFactor, raw_value: f64) -> TrustEvidence {
    TrustEvidence { factor, raw_value }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scorer() -> SybilScorer {
        SybilScorer::new()
    }

    // ── Normalization ───────────────────────────────────────────────

    #[test]
    fn normalize_zero_is_zero() {
        let s = scorer();
        assert_eq!(s.normalize_factor(&TrustFactor::IdentityAge, 0.0), 0.0);
        assert_eq!(s.normalize_factor(&TrustFactor::SocialVouches, 0.0), 0.0);
    }

    #[test]
    fn normalize_negative_is_zero() {
        let s = scorer();
        assert_eq!(s.normalize_factor(&TrustFactor::IdentityAge, -10.0), 0.0);
    }

    #[test]
    fn normalize_max_is_one() {
        let s = scorer();
        let score = s.normalize_factor(&TrustFactor::IdentityAge, 365.0);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn normalize_over_max_capped() {
        let s = scorer();
        let score = s.normalize_factor(&TrustFactor::IdentityAge, 1000.0);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn normalize_diminishing_returns() {
        let s = scorer();
        // Half the max should give sqrt(0.5) ≈ 0.707
        let score = s.normalize_factor(&TrustFactor::IdentityAge, 182.5);
        assert!((score - 0.707).abs() < 0.01);
    }

    #[test]
    fn normalize_quarter_max() {
        let s = scorer();
        // Quarter max → sqrt(0.25) = 0.5
        let score = s.normalize_factor(&TrustFactor::IdentityAge, 91.25);
        assert!((score - 0.5).abs() < 0.01);
    }

    #[test]
    fn normalize_device_binding_binary() {
        let s = scorer();
        assert_eq!(s.normalize_factor(&TrustFactor::DeviceBinding, 0.0), 0.0);
        assert_eq!(s.normalize_factor(&TrustFactor::DeviceBinding, 0.5), 0.0);
        assert_eq!(s.normalize_factor(&TrustFactor::DeviceBinding, 1.0), 1.0);
        assert_eq!(s.normalize_factor(&TrustFactor::DeviceBinding, 5.0), 1.0);
    }

    // ── Scoring ─────────────────────────────────────────────────────

    #[test]
    fn score_no_evidence() {
        let s = scorer();
        let result = s.score("did:key:z6MkEmpty", &[]);
        assert_eq!(result.score, 0.0);
        assert!(!result.eligible);
    }

    #[test]
    fn score_single_factor() {
        let s = scorer();
        let result = s.score(
            "did:key:z6MkAlice",
            &[evidence(TrustFactor::IdentityAge, 365.0)],
        );

        // 1.0 * 0.15 (identity age weight) = 0.15
        assert!((result.score - 0.15).abs() < 0.01);
        assert!(result.factors.contains_key(&TrustFactor::IdentityAge));
    }

    #[test]
    fn score_all_factors_max() {
        let s = scorer();
        let result = s.score(
            "did:key:z6MkFull",
            &[
                evidence(TrustFactor::IdentityAge, 365.0),
                evidence(TrustFactor::SocialVouches, 10.0),
                evidence(TrustFactor::OnChainActivity, 100.0),
                evidence(TrustFactor::StakeAmount, 10_000.0),
                evidence(TrustFactor::DeviceBinding, 1.0),
                evidence(TrustFactor::CredentialCount, 5.0),
            ],
        );

        // All maxed out → score ≈ 1.0
        assert!((result.score - 1.0).abs() < 0.01);
        assert!(result.eligible);
    }

    #[test]
    fn score_moderate_evidence() {
        let s = scorer();
        let result = s.score(
            "did:key:z6MkBob",
            &[
                evidence(TrustFactor::IdentityAge, 90.0),     // ~0.50
                evidence(TrustFactor::SocialVouches, 3.0),    // ~0.55
                evidence(TrustFactor::OnChainActivity, 20.0), // ~0.45
            ],
        );

        // Weighted: 0.50*0.15 + 0.55*0.25 + 0.45*0.20 ≈ 0.075 + 0.137 + 0.089 ≈ 0.30
        assert!(result.score > 0.2);
        assert!(result.score < 0.5);
    }

    #[test]
    fn score_did_preserved() {
        let s = scorer();
        let result = s.score("did:key:z6MkTest", &[]);
        assert_eq!(result.did, "did:key:z6MkTest");
    }

    // ── Eligibility ─────────────────────────────────────────────────

    #[test]
    fn eligible_above_threshold() {
        let s = scorer();
        let result = s.score(
            "did:key:z6MkGood",
            &[
                evidence(TrustFactor::IdentityAge, 365.0),
                evidence(TrustFactor::SocialVouches, 10.0),
                evidence(TrustFactor::OnChainActivity, 100.0),
            ],
        );

        assert!(result.eligible);
    }

    #[test]
    fn ineligible_below_threshold() {
        let s = scorer();
        let result = s.score(
            "did:key:z6MkNew",
            &[evidence(TrustFactor::IdentityAge, 1.0)], // very new
        );

        assert!(!result.eligible);
    }

    #[test]
    fn is_eligible_convenience() {
        let s = scorer();
        assert!(!s.is_eligible("did:key:z6MkNew", &[]));
        assert!(s.is_eligible(
            "did:key:z6MkGood",
            &[
                evidence(TrustFactor::SocialVouches, 10.0),
                evidence(TrustFactor::OnChainActivity, 100.0),
            ]
        ));
    }

    #[test]
    fn custom_threshold() {
        let mut s = scorer();
        s.set_threshold(0.9);
        assert_eq!(s.threshold(), 0.9);

        // Even moderate evidence fails a high threshold
        assert!(!s.is_eligible(
            "did:key:z6MkModerate",
            &[
                evidence(TrustFactor::IdentityAge, 180.0),
                evidence(TrustFactor::SocialVouches, 5.0),
            ]
        ));
    }

    #[test]
    fn threshold_clamped() {
        let mut s = scorer();
        s.set_threshold(2.0);
        assert_eq!(s.threshold(), 1.0);

        s.set_threshold(-1.0);
        assert_eq!(s.threshold(), 0.0);
    }

    // ── Weights ─────────────────────────────────────────────────────

    #[test]
    fn default_weights_sum_to_one() {
        let w = ScorerWeights::default_weights();
        assert!((w.total() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn normalized_weight() {
        let w = ScorerWeights::default_weights();
        let n = w.normalized(&TrustFactor::SocialVouches);
        assert!((n - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn custom_weights() {
        let mut weights = HashMap::new();
        weights.insert(TrustFactor::StakeAmount, 1.0);

        let s = SybilScorer::with_weights(ScorerWeights::custom(weights));
        let result = s.score(
            "did:key:z6MkStaker",
            &[evidence(TrustFactor::StakeAmount, 10_000.0)],
        );

        // Only stake matters, maxed out → score = 1.0
        assert!((result.score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn custom_weights_normalized() {
        let mut weights = HashMap::new();
        weights.insert(TrustFactor::StakeAmount, 2.0);
        weights.insert(TrustFactor::IdentityAge, 2.0);

        let w = ScorerWeights::custom(weights);
        // Each is 50% of total
        assert!((w.normalized(&TrustFactor::StakeAmount) - 0.5).abs() < f64::EPSILON);
        assert!((w.normalized(&TrustFactor::IdentityAge) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn missing_weight_is_zero() {
        let w = ScorerWeights::custom(HashMap::new());
        assert_eq!(w.get(&TrustFactor::IdentityAge), 0.0);
    }

    // ── Batch operations ────────────────────────────────────────────

    #[test]
    fn score_batch() {
        let s = scorer();
        let mut identities = HashMap::new();
        identities.insert(
            "alice".to_string(),
            vec![evidence(TrustFactor::IdentityAge, 365.0)],
        );
        identities.insert(
            "bob".to_string(),
            vec![evidence(TrustFactor::SocialVouches, 10.0)],
        );

        let scores = s.score_batch(&identities);
        assert_eq!(scores.len(), 2);
        assert!(scores.contains_key("alice"));
        assert!(scores.contains_key("bob"));
    }

    #[test]
    fn filter_eligible() {
        let s = scorer();
        let mut identities = HashMap::new();

        // High evidence — eligible
        identities.insert(
            "alice".to_string(),
            vec![
                evidence(TrustFactor::IdentityAge, 365.0),
                evidence(TrustFactor::SocialVouches, 10.0),
                evidence(TrustFactor::OnChainActivity, 100.0),
            ],
        );

        // No evidence — not eligible
        identities.insert("bob".to_string(), vec![]);

        let eligible = s.filter_eligible(&identities);
        assert_eq!(eligible.len(), 1);
        assert!(eligible.contains(&"alice".to_string()));
    }

    // ── Normalization config ────────────────────────────────────────

    #[test]
    fn custom_normalization() {
        let mut s = scorer();
        s.set_normalization(NormalizationConfig {
            age_max_days: 30.0, // Only 30 days for max age score
            ..Default::default()
        });

        let result = s.score(
            "did:key:z6MkFast",
            &[evidence(TrustFactor::IdentityAge, 30.0)],
        );

        let age_score = result.factors.get(&TrustFactor::IdentityAge).unwrap();
        assert!((age_score - 1.0).abs() < f64::EPSILON);
    }

    // ── Edge cases ──────────────────────────────────────────────────

    #[test]
    fn duplicate_factors_last_wins() {
        let s = scorer();
        let result = s.score(
            "did:key:z6MkDup",
            &[
                evidence(TrustFactor::IdentityAge, 30.0),
                evidence(TrustFactor::IdentityAge, 365.0), // overwrites
            ],
        );

        let age = result.factors.get(&TrustFactor::IdentityAge).unwrap();
        assert!((age - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn score_is_deterministic() {
        let s = scorer();
        let ev = vec![
            evidence(TrustFactor::IdentityAge, 100.0),
            evidence(TrustFactor::SocialVouches, 5.0),
        ];

        let s1 = s.score("did:key:z6MkDet", &ev);
        let s2 = s.score("did:key:z6MkDet", &ev);
        assert!((s1.score - s2.score).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_weights_zero_score() {
        let s = SybilScorer::with_weights(ScorerWeights::custom(HashMap::new()));
        let result = s.score(
            "did:key:z6MkZero",
            &[evidence(TrustFactor::IdentityAge, 365.0)],
        );
        assert_eq!(result.score, 0.0);
    }

    // ── Serialization ───────────────────────────────────────────────

    #[test]
    fn sybil_score_serializes() {
        let s = scorer();
        let result = s.score(
            "did:key:z6MkSer",
            &[evidence(TrustFactor::IdentityAge, 100.0)],
        );

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: SybilScore = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.did, "did:key:z6MkSer");
        assert!((deserialized.score - result.score).abs() < f64::EPSILON);
    }

    #[test]
    fn trust_evidence_serializes() {
        let ev = evidence(TrustFactor::SocialVouches, 7.0);
        let json = serde_json::to_string(&ev).unwrap();
        let deserialized: TrustEvidence = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.factor, TrustFactor::SocialVouches);
        assert!((deserialized.raw_value - 7.0).abs() < f64::EPSILON);
    }

    #[test]
    fn scorer_weights_serializes() {
        let w = ScorerWeights::default_weights();
        let json = serde_json::to_string(&w).unwrap();
        let deserialized: ScorerWeights = serde_json::from_str(&json).unwrap();
        assert!((deserialized.total() - 1.0).abs() < 1e-10);
    }

    // ── Evidence helper ─────────────────────────────────────────────

    #[test]
    fn evidence_helper() {
        let ev = evidence(TrustFactor::StakeAmount, 5000.0);
        assert_eq!(ev.factor, TrustFactor::StakeAmount);
        assert!((ev.raw_value - 5000.0).abs() < f64::EPSILON);
    }
}
