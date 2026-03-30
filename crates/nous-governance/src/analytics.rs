//! Governance analytics — dashboard-ready data aggregation.
//!
//! Provides computed views over DAOs, proposals, votes, delegations, and
//! treasury data. Designed for use by the web app governance dashboard
//! and the API layer.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::dao::{Dao, MemberRole};
use crate::delegation::{DelegationRegistry, DelegationScope};
use crate::proposal::{Proposal, ProposalStatus};
use crate::treasury::Treasury;
use crate::vote::VoteTally;

/// Summary of a DAO for dashboard display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaoSummary {
    pub id: String,
    pub member_count: usize,
    pub admin_count: usize,
    pub total_credits: u64,
    pub default_quorum: f64,
    pub default_threshold: f64,
}

/// Summary of a proposal for listing/overview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalSummary {
    pub id: String,
    pub title: String,
    pub status: ProposalStatus,
    pub proposer_did: String,
    pub votes_for: u64,
    pub votes_against: u64,
    pub votes_abstain: u64,
    pub total_voters: usize,
    pub participation_rate: f64,
    pub approval_rate: f64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Power distribution across DAO members.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerDistribution {
    /// Member DID → effective voting power after delegation.
    pub effective_power: HashMap<String, u64>,
    /// Total voting power in the DAO.
    pub total_power: u64,
    /// Gini coefficient of power distribution (0 = equal, 1 = concentrated).
    pub gini_coefficient: f64,
    /// Top holders (DID, power) sorted by power descending.
    pub top_holders: Vec<(String, u64)>,
    /// Number of members with delegated power (they delegated to someone else).
    pub delegators: usize,
}

/// Treasury summary for dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasurySummary {
    pub dao_id: String,
    pub balances: HashMap<String, u128>,
    pub total_proposals: usize,
    pub pending_proposals: usize,
    pub total_spent: HashMap<String, u128>,
}

/// Activity feed item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityItem {
    pub timestamp: DateTime<Utc>,
    pub kind: ActivityKind,
    pub actor: String,
    pub description: String,
}

/// Kinds of governance activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityKind {
    ProposalCreated,
    VoteCast,
    ProposalPassed,
    ProposalRejected,
    DelegationCreated,
    TreasuryDeposit,
    TreasurySpend,
}

/// Compute a DAO summary from a DAO instance.
pub fn summarize_dao(dao: &Dao) -> DaoSummary {
    let mut admin_count = 0;
    let mut total_credits = 0u64;

    for member in dao.members.values() {
        total_credits += member.credits;
        if member.role == MemberRole::Admin || member.role == MemberRole::Founder {
            admin_count += 1;
        }
    }

    DaoSummary {
        id: dao.id.clone(),
        member_count: dao.member_count(),
        admin_count,
        total_credits,
        default_quorum: dao.default_quorum,
        default_threshold: dao.default_threshold,
    }
}

/// Compute a proposal summary with vote tally.
pub fn summarize_proposal(
    proposal: &Proposal,
    tally: Option<&VoteTally>,
    eligible_voters: usize,
) -> ProposalSummary {
    let (votes_for, votes_against, votes_abstain, total_voters) = tally
        .map(|t| {
            let result = t.tally(eligible_voters);
            (
                result.votes_for,
                result.votes_against,
                result.votes_abstain,
                result.total_voters,
            )
        })
        .unwrap_or((0, 0, 0, 0));

    let total_votes = votes_for + votes_against + votes_abstain;
    let participation_rate = if eligible_voters > 0 {
        total_voters as f64 / eligible_voters as f64
    } else {
        0.0
    };
    let approval_rate = if total_votes > 0 {
        votes_for as f64 / total_votes as f64
    } else {
        0.0
    };

    ProposalSummary {
        id: proposal.id.clone(),
        title: proposal.title.clone(),
        status: proposal.status,
        proposer_did: proposal.proposer_did.clone(),
        votes_for,
        votes_against,
        votes_abstain,
        total_voters,
        participation_rate,
        approval_rate,
        is_active: proposal.is_voting_active(),
        created_at: proposal.voting_starts,
    }
}

/// Compute power distribution with delegation resolution.
pub fn compute_power_distribution(dao: &Dao, registry: &DelegationRegistry) -> PowerDistribution {
    let mut base_power: HashMap<String, u64> = HashMap::new();
    for (did, member) in &dao.members {
        base_power.insert(did.clone(), member.credits);
    }

    let scope = DelegationScope::Dao(dao.id.clone());
    let effective = registry.effective_power(&base_power, &scope);
    let total_power: u64 = effective.values().sum();

    let mut sorted: Vec<(String, u64)> = effective.iter().map(|(k, &v)| (k.clone(), v)).collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let gini = compute_gini(&sorted.iter().map(|(_, v)| *v).collect::<Vec<_>>());

    let delegators = base_power.len() - effective.len();

    PowerDistribution {
        effective_power: effective,
        total_power,
        gini_coefficient: gini,
        top_holders: sorted.into_iter().take(10).collect(),
        delegators,
    }
}

/// Compute treasury summary.
pub fn summarize_treasury(treasury: &Treasury) -> TreasurySummary {
    TreasurySummary {
        dao_id: treasury.dao_id.clone(),
        balances: treasury.balances.clone(),
        total_proposals: treasury.proposals.len(),
        pending_proposals: treasury.pending_proposals().len(),
        total_spent: treasury.total_spent.clone(),
    }
}

/// Compute the Gini coefficient for a set of values.
/// Returns 0.0 for perfect equality, 1.0 for maximum inequality.
fn compute_gini(values: &[u64]) -> f64 {
    let n = values.len();
    if n <= 1 {
        return 0.0;
    }

    let total: f64 = values.iter().map(|&v| v as f64).sum();
    if total == 0.0 {
        return 0.0;
    }

    let mut sum_of_abs_diffs = 0.0;
    for &a in values {
        for &b in values {
            sum_of_abs_diffs += (a as f64 - b as f64).abs();
        }
    }

    sum_of_abs_diffs / (2.0 * n as f64 * total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dao::Dao;
    use crate::vote::VoteChoice;

    fn test_dao() -> Dao {
        let mut dao = Dao::create("did:key:founder", "Test DAO", "A test DAO");
        dao.add_member("did:key:alice").unwrap();
        dao.add_member("did:key:bob").unwrap();
        dao.add_member("did:key:carol").unwrap();
        // Grant extra credits for interesting power distribution.
        dao.grant_credits("did:key:alice", 40).unwrap();
        dao.grant_credits("did:key:bob", 20).unwrap();
        dao.grant_credits("did:key:carol", 10).unwrap();
        dao
    }

    #[test]
    fn dao_summary() {
        let dao = test_dao();
        let summary = summarize_dao(&dao);

        assert_eq!(summary.member_count, 4);
        assert_eq!(summary.admin_count, 1); // founder
        assert!(summary.total_credits > 0);
    }

    #[test]
    fn dao_summary_empty() {
        let dao = Dao::create("did:key:founder", "Empty", "An empty DAO");
        let summary = summarize_dao(&dao);
        assert_eq!(summary.member_count, 1); // Just founder.
    }

    #[test]
    fn proposal_summary_no_votes() {
        let identity = nous_identity::Identity::generate();
        let proposal = crate::ProposalBuilder::new("dao-1", "Test Prop", "Description")
            .submit(&identity)
            .unwrap();

        let summary = summarize_proposal(&proposal, None, 10);
        assert_eq!(summary.votes_for, 0);
        assert_eq!(summary.total_voters, 0);
        assert_eq!(summary.participation_rate, 0.0);
    }

    #[test]
    fn proposal_summary_with_votes() {
        let identity = nous_identity::Identity::generate();
        let proposal = crate::ProposalBuilder::new("dao-1", "Test", "Desc")
            .submit(&identity)
            .unwrap();

        let voter1 = nous_identity::Identity::generate();
        let voter2 = nous_identity::Identity::generate();

        let mut tally = VoteTally::new(&proposal.id, 0.1, 0.5);
        let ballot1 = crate::Ballot::new(&proposal.id, &voter1, VoteChoice::For, 9).unwrap();
        let ballot2 = crate::Ballot::new(&proposal.id, &voter2, VoteChoice::Against, 4).unwrap();
        tally.cast(ballot1).unwrap();
        tally.cast(ballot2).unwrap();

        let summary = summarize_proposal(&proposal, Some(&tally), 10);
        assert!(summary.votes_for > 0);
        assert!(summary.votes_against > 0);
        assert_eq!(summary.total_voters, 2);
        assert!((summary.participation_rate - 0.2).abs() < 0.01);
    }

    #[test]
    fn power_distribution_no_delegation() {
        let dao = test_dao();
        let registry = DelegationRegistry::new();
        let dist = compute_power_distribution(&dao, &registry);

        assert_eq!(dist.effective_power.len(), 4);
        assert!(dist.total_power > 0);
        assert_eq!(dist.delegators, 0);
    }

    #[test]
    fn power_distribution_with_delegation() {
        let dao = test_dao();
        let mut registry = DelegationRegistry::new();

        // Alice delegates to Bob.
        registry
            .delegate(
                "did:key:alice",
                "did:key:bob",
                crate::DelegationScope::Dao("test".into()),
                None,
            )
            .unwrap();

        let dist = compute_power_distribution(&dao, &registry);

        // Bob should have alice's power + his own.
        let bob_power = dist
            .effective_power
            .get("did:key:bob")
            .copied()
            .unwrap_or(0);
        assert!(bob_power >= 30); // At least bob's original.
    }

    #[test]
    fn gini_coefficient_equal() {
        let values = vec![100, 100, 100, 100];
        let gini = compute_gini(&values);
        assert!(gini.abs() < 0.001);
    }

    #[test]
    fn gini_coefficient_unequal() {
        let values = vec![0, 0, 0, 1000];
        let gini = compute_gini(&values);
        assert!(gini > 0.5);
    }

    #[test]
    fn gini_coefficient_single() {
        let gini = compute_gini(&[100]);
        assert_eq!(gini, 0.0);
    }

    #[test]
    fn gini_coefficient_empty() {
        let gini = compute_gini(&[]);
        assert_eq!(gini, 0.0);
    }

    #[test]
    fn treasury_summary() {
        let mut treasury = Treasury::new("dao-1");
        treasury.deposit("ETH", 1_000_000);
        treasury.deposit("USDC", 5_000_000);

        let summary = summarize_treasury(&treasury);
        assert_eq!(summary.dao_id, "dao-1");
        assert_eq!(summary.balances.len(), 2);
        assert_eq!(summary.pending_proposals, 0);
    }

    #[test]
    fn activity_item_serializes() {
        let item = ActivityItem {
            timestamp: Utc::now(),
            kind: ActivityKind::ProposalCreated,
            actor: "did:key:alice".into(),
            description: "Created proposal P-42".into(),
        };
        let json = serde_json::to_string(&item).unwrap();
        let restored: ActivityItem = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.actor, "did:key:alice");
    }

    #[test]
    fn dao_summary_serializes() {
        let dao = test_dao();
        let summary = summarize_dao(&dao);
        let json = serde_json::to_string(&summary).unwrap();
        let restored: DaoSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.member_count, 4);
    }

    #[test]
    fn power_distribution_serializes() {
        let dao = test_dao();
        let registry = DelegationRegistry::new();
        let dist = compute_power_distribution(&dao, &registry);
        let json = serde_json::to_string(&dist).unwrap();
        let restored: PowerDistribution = serde_json::from_str(&json).unwrap();
        assert!(restored.total_power > 0);
    }

    #[test]
    fn top_holders_sorted() {
        let dao = test_dao();
        let registry = DelegationRegistry::new();
        let dist = compute_power_distribution(&dao, &registry);

        for i in 1..dist.top_holders.len() {
            assert!(dist.top_holders[i - 1].1 >= dist.top_holders[i].1);
        }
    }
}
