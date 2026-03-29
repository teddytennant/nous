use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use nous_core::{Error, Result};

/// Maximum depth of transitive delegation chains.
/// Prevents unbounded traversal and limits gas-like cost of power computation.
const MAX_CHAIN_DEPTH: usize = 10;

/// Scope of a delegation — either all proposals within a DAO, or a single proposal.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DelegationScope {
    /// Delegation applies to all proposals in the DAO.
    Dao(String),
    /// Delegation applies only to a specific proposal.
    Proposal(String),
}

/// A single delegation: one principal delegates their voting power to one delegate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delegation {
    pub id: String,
    pub from_did: String,
    pub to_did: String,
    pub scope: DelegationScope,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked: bool,
}

impl Delegation {
    fn is_active(&self) -> bool {
        if self.revoked {
            return false;
        }
        if let Some(expires) = self.expires_at {
            return Utc::now() < expires;
        }
        true
    }
}

/// Registry that tracks all delegations and computes effective voting power.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DelegationRegistry {
    /// All delegations indexed by ID.
    delegations: HashMap<String, Delegation>,
    /// Index: "from_did\0scope_json" → delegation_id for fast lookup and uniqueness.
    #[serde(skip)]
    by_delegator: HashMap<String, String>,
}

impl DelegationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuild the `by_delegator` index from the delegations map.
    /// Called after deserialization since the index is skipped in serde.
    pub fn rebuild_index(&mut self) {
        self.by_delegator.clear();
        for (id, d) in &self.delegations {
            let key = Self::delegator_key(&d.from_did, &d.scope);
            self.by_delegator.insert(key, id.clone());
        }
    }

    fn delegator_key(did: &str, scope: &DelegationScope) -> String {
        match scope {
            DelegationScope::Dao(id) => format!("{did}\0dao:{id}"),
            DelegationScope::Proposal(id) => format!("{did}\0prop:{id}"),
        }
    }

    /// Create a new delegation. A principal can only have one active delegation per scope.
    /// Returns an error if:
    /// - Self-delegation is attempted
    /// - An active delegation already exists for this (from, scope)
    /// - The delegation would create a cycle
    pub fn delegate(
        &mut self,
        from_did: impl Into<String>,
        to_did: impl Into<String>,
        scope: DelegationScope,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<String> {
        let from_did = from_did.into();
        let to_did = to_did.into();

        if from_did == to_did {
            return Err(Error::Governance("cannot delegate to self".into()));
        }

        // Check for existing active delegation from this principal in this scope
        let key = Self::delegator_key(&from_did, &scope);
        if let Some(existing_id) = self.by_delegator.get(&key)
            && let Some(existing) = self.delegations.get(existing_id)
            && existing.is_active()
        {
            return Err(Error::Governance(
                "active delegation already exists for this scope; revoke it first".into(),
            ));
        }

        // Check for cycles: walk the chain from to_did forward. If we reach from_did, it's a cycle.
        if self.would_create_cycle(&from_did, &to_did, &scope) {
            return Err(Error::Governance("delegation would create a cycle".into()));
        }

        let id = format!("del:{}", Uuid::new_v4());
        let delegation = Delegation {
            id: id.clone(),
            from_did: from_did.clone(),
            to_did,
            scope: scope.clone(),
            created_at: Utc::now(),
            expires_at,
            revoked: false,
        };

        self.delegations.insert(id.clone(), delegation);
        self.by_delegator.insert(key, id.clone());
        Ok(id)
    }

    /// Revoke a delegation. Only the delegator (from_did) can revoke.
    pub fn revoke(&mut self, delegation_id: &str, requester_did: &str) -> Result<()> {
        let delegation = self
            .delegations
            .get_mut(delegation_id)
            .ok_or_else(|| Error::NotFound("delegation not found".into()))?;

        if delegation.from_did != requester_did {
            return Err(Error::PermissionDenied(
                "only the delegator can revoke".into(),
            ));
        }

        if delegation.revoked {
            return Err(Error::Governance("delegation already revoked".into()));
        }

        delegation.revoked = true;
        Ok(())
    }

    /// Get a delegation by ID.
    pub fn get(&self, delegation_id: &str) -> Option<&Delegation> {
        self.delegations.get(delegation_id)
    }

    /// List all active delegations for a given scope.
    pub fn active_delegations(&self, scope: &DelegationScope) -> Vec<&Delegation> {
        self.delegations
            .values()
            .filter(|d| d.scope == *scope && d.is_active())
            .collect()
    }

    /// List all active delegations made BY a specific DID.
    pub fn delegations_from(&self, did: &str) -> Vec<&Delegation> {
        self.delegations
            .values()
            .filter(|d| d.from_did == did && d.is_active())
            .collect()
    }

    /// List all active delegations received BY a specific DID.
    pub fn delegations_to(&self, did: &str) -> Vec<&Delegation> {
        self.delegations
            .values()
            .filter(|d| d.to_did == did && d.is_active())
            .collect()
    }

    /// Resolve the final delegate in a transitive chain for a given principal and scope.
    /// Returns the DID that will actually cast the vote.
    /// If there are no delegations, returns None (the principal votes for themselves).
    pub fn resolve_delegate(&self, from_did: &str, scope: &DelegationScope) -> Option<String> {
        let mut current = from_did.to_string();
        let mut visited = HashSet::new();
        visited.insert(current.clone());

        for _ in 0..MAX_CHAIN_DEPTH {
            match self.active_delegation_from(&current, scope) {
                Some(d) => {
                    if !visited.insert(d.to_did.clone()) {
                        // Cycle detected in resolution (shouldn't happen if delegate() is correct)
                        return None;
                    }
                    current = d.to_did.clone();
                }
                None => break,
            }
        }

        if current == from_did {
            None
        } else {
            Some(current)
        }
    }

    /// Compute effective voting power for all members in a scope.
    /// Members who have delegated their power to someone else contribute their credits
    /// transitively to the final delegate in the chain.
    ///
    /// Returns a map of DID → effective credits (only for members who retain or receive power).
    pub fn effective_power(
        &self,
        member_credits: &HashMap<String, u64>,
        scope: &DelegationScope,
    ) -> HashMap<String, u64> {
        let mut power: HashMap<String, u64> = HashMap::new();

        for (did, &credits) in member_credits {
            if credits == 0 {
                continue;
            }

            // Resolve: where does this member's power end up?
            match self.resolve_delegate(did, scope) {
                Some(final_delegate) => {
                    // Power flows to the final delegate
                    *power.entry(final_delegate).or_insert(0) += credits;
                }
                None => {
                    // Member votes for themselves
                    *power.entry(did.clone()).or_insert(0) += credits;
                }
            }
        }

        power
    }

    /// Get the full delegation chain starting from a DID.
    /// Returns the ordered list of DIDs: [from, delegate1, delegate2, ..., final].
    pub fn delegation_chain(&self, from_did: &str, scope: &DelegationScope) -> Vec<String> {
        let mut chain = vec![from_did.to_string()];
        let mut current = from_did.to_string();
        let mut visited = HashSet::new();
        visited.insert(current.clone());

        for _ in 0..MAX_CHAIN_DEPTH {
            match self.active_delegation_from(&current, scope) {
                Some(d) => {
                    if !visited.insert(d.to_did.clone()) {
                        break;
                    }
                    chain.push(d.to_did.clone());
                    current = d.to_did.clone();
                }
                None => break,
            }
        }

        chain
    }

    /// Total number of delegations (including revoked/expired).
    pub fn total_delegations(&self) -> usize {
        self.delegations.len()
    }

    // ── Private helpers ──────────────────────────────────────────────────

    /// Find the active delegation FROM a specific DID in a given scope.
    /// Checks both exact scope match and DAO-wide delegation (which covers proposals).
    fn active_delegation_from(&self, did: &str, scope: &DelegationScope) -> Option<&Delegation> {
        // First check exact scope match
        let key = Self::delegator_key(did, scope);
        if let Some(id) = self.by_delegator.get(&key)
            && let Some(d) = self.delegations.get(id)
            && d.is_active()
        {
            return Some(d);
        }

        // If scope is a proposal, also check DAO-wide delegation
        if let DelegationScope::Proposal(prop_id) = scope {
            // We need to find the DAO for this proposal — but we don't have that info here.
            // For now, proposal-scoped delegations only match exactly.
            // DAO-wide delegations are matched when scope is Dao(_).
            let _ = prop_id;
        }

        None
    }

    /// Check whether adding from→to would create a cycle.
    /// Walk forward from `to_did`: if we ever reach `from_did`, there's a cycle.
    fn would_create_cycle(&self, from_did: &str, to_did: &str, scope: &DelegationScope) -> bool {
        let mut current = to_did.to_string();
        let mut visited = HashSet::new();
        visited.insert(from_did.to_string());
        visited.insert(to_did.to_string());

        for _ in 0..MAX_CHAIN_DEPTH {
            match self.active_delegation_from(&current, scope) {
                Some(d) => {
                    if d.to_did == from_did {
                        return true;
                    }
                    if !visited.insert(d.to_did.clone()) {
                        // Hit a node we already saw (but not from_did) — no cycle through from_did
                        return false;
                    }
                    current = d.to_did.clone();
                }
                None => return false,
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> DelegationScope {
        DelegationScope::Dao("dao:test".into())
    }

    fn prop_scope() -> DelegationScope {
        DelegationScope::Proposal("prop:test".into())
    }

    #[test]
    fn simple_delegation() {
        let mut reg = DelegationRegistry::new();
        let id = reg.delegate("alice", "bob", scope(), None).unwrap();

        assert!(id.starts_with("del:"));
        let d = reg.get(&id).unwrap();
        assert_eq!(d.from_did, "alice");
        assert_eq!(d.to_did, "bob");
        assert!(d.is_active());
    }

    #[test]
    fn self_delegation_rejected() {
        let mut reg = DelegationRegistry::new();
        let result = reg.delegate("alice", "alice", scope(), None);
        assert!(result.is_err());
    }

    #[test]
    fn duplicate_delegation_rejected() {
        let mut reg = DelegationRegistry::new();
        reg.delegate("alice", "bob", scope(), None).unwrap();
        let result = reg.delegate("alice", "carol", scope(), None);
        assert!(result.is_err());
    }

    #[test]
    fn different_scope_allowed() {
        let mut reg = DelegationRegistry::new();
        reg.delegate("alice", "bob", scope(), None).unwrap();
        // Same delegator, different scope — allowed
        reg.delegate("alice", "carol", prop_scope(), None).unwrap();
        assert_eq!(reg.total_delegations(), 2);
    }

    #[test]
    fn revoke_delegation() {
        let mut reg = DelegationRegistry::new();
        let id = reg.delegate("alice", "bob", scope(), None).unwrap();

        reg.revoke(&id, "alice").unwrap();
        let d = reg.get(&id).unwrap();
        assert!(d.revoked);
        assert!(!d.is_active());
    }

    #[test]
    fn revoke_only_by_delegator() {
        let mut reg = DelegationRegistry::new();
        let id = reg.delegate("alice", "bob", scope(), None).unwrap();

        let result = reg.revoke(&id, "bob");
        assert!(result.is_err());
    }

    #[test]
    fn double_revoke_rejected() {
        let mut reg = DelegationRegistry::new();
        let id = reg.delegate("alice", "bob", scope(), None).unwrap();
        reg.revoke(&id, "alice").unwrap();
        let result = reg.revoke(&id, "alice");
        assert!(result.is_err());
    }

    #[test]
    fn revoke_then_redelegate() {
        let mut reg = DelegationRegistry::new();
        let id = reg.delegate("alice", "bob", scope(), None).unwrap();
        reg.revoke(&id, "alice").unwrap();

        // Can now delegate to someone else
        let id2 = reg.delegate("alice", "carol", scope(), None).unwrap();
        let d = reg.get(&id2).unwrap();
        assert_eq!(d.to_did, "carol");
    }

    #[test]
    fn cycle_detection_direct() {
        let mut reg = DelegationRegistry::new();
        reg.delegate("alice", "bob", scope(), None).unwrap();
        // bob→alice would create alice→bob→alice cycle
        let result = reg.delegate("bob", "alice", scope(), None);
        assert!(result.is_err());
    }

    #[test]
    fn cycle_detection_transitive() {
        let mut reg = DelegationRegistry::new();
        reg.delegate("alice", "bob", scope(), None).unwrap();
        reg.delegate("bob", "carol", scope(), None).unwrap();
        // carol→alice would create alice→bob→carol→alice cycle
        let result = reg.delegate("carol", "alice", scope(), None);
        assert!(result.is_err());
    }

    #[test]
    fn no_false_cycle_detection() {
        let mut reg = DelegationRegistry::new();
        reg.delegate("alice", "bob", scope(), None).unwrap();
        reg.delegate("carol", "bob", scope(), None).unwrap();
        // dave→alice is fine, no cycle
        reg.delegate("dave", "alice", scope(), None).unwrap();
        assert_eq!(reg.total_delegations(), 3);
    }

    #[test]
    fn resolve_delegate_direct() {
        let mut reg = DelegationRegistry::new();
        reg.delegate("alice", "bob", scope(), None).unwrap();

        assert_eq!(reg.resolve_delegate("alice", &scope()), Some("bob".into()));
        // Bob has no delegation — votes for himself
        assert_eq!(reg.resolve_delegate("bob", &scope()), None);
    }

    #[test]
    fn resolve_delegate_transitive() {
        let mut reg = DelegationRegistry::new();
        reg.delegate("alice", "bob", scope(), None).unwrap();
        reg.delegate("bob", "carol", scope(), None).unwrap();

        assert_eq!(
            reg.resolve_delegate("alice", &scope()),
            Some("carol".into())
        );
        assert_eq!(reg.resolve_delegate("bob", &scope()), Some("carol".into()));
    }

    #[test]
    fn delegation_chain_linear() {
        let mut reg = DelegationRegistry::new();
        reg.delegate("alice", "bob", scope(), None).unwrap();
        reg.delegate("bob", "carol", scope(), None).unwrap();
        reg.delegate("carol", "dave", scope(), None).unwrap();

        let chain = reg.delegation_chain("alice", &scope());
        assert_eq!(chain, vec!["alice", "bob", "carol", "dave"]);
    }

    #[test]
    fn effective_power_no_delegations() {
        let reg = DelegationRegistry::new();
        let mut credits = HashMap::new();
        credits.insert("alice".to_string(), 100u64);
        credits.insert("bob".to_string(), 50u64);

        let power = reg.effective_power(&credits, &scope());
        assert_eq!(power.get("alice"), Some(&100));
        assert_eq!(power.get("bob"), Some(&50));
    }

    #[test]
    fn effective_power_simple_delegation() {
        let mut reg = DelegationRegistry::new();
        reg.delegate("alice", "bob", scope(), None).unwrap();

        let mut credits = HashMap::new();
        credits.insert("alice".to_string(), 100u64);
        credits.insert("bob".to_string(), 50u64);

        let power = reg.effective_power(&credits, &scope());
        // Alice's 100 flows to Bob. Bob has his own 50.
        assert_eq!(power.get("bob"), Some(&150));
        assert!(power.get("alice").is_none());
    }

    #[test]
    fn effective_power_transitive() {
        let mut reg = DelegationRegistry::new();
        reg.delegate("alice", "bob", scope(), None).unwrap();
        reg.delegate("bob", "carol", scope(), None).unwrap();

        let mut credits = HashMap::new();
        credits.insert("alice".to_string(), 100u64);
        credits.insert("bob".to_string(), 50u64);
        credits.insert("carol".to_string(), 25u64);

        let power = reg.effective_power(&credits, &scope());
        // Alice → Bob → Carol: all power accumulates at Carol
        assert_eq!(power.get("carol"), Some(&175)); // 100 + 50 + 25
        assert!(power.get("alice").is_none());
        assert!(power.get("bob").is_none());
    }

    #[test]
    fn effective_power_fan_in() {
        let mut reg = DelegationRegistry::new();
        reg.delegate("alice", "dave", scope(), None).unwrap();
        reg.delegate("bob", "dave", scope(), None).unwrap();
        reg.delegate("carol", "dave", scope(), None).unwrap();

        let mut credits = HashMap::new();
        credits.insert("alice".to_string(), 10u64);
        credits.insert("bob".to_string(), 20u64);
        credits.insert("carol".to_string(), 30u64);
        credits.insert("dave".to_string(), 5u64);

        let power = reg.effective_power(&credits, &scope());
        assert_eq!(power.get("dave"), Some(&65)); // 10 + 20 + 30 + 5
    }

    #[test]
    fn expired_delegation_inactive() {
        let mut reg = DelegationRegistry::new();
        let past = Utc::now() - chrono::Duration::hours(1);
        reg.delegate("alice", "bob", scope(), Some(past)).unwrap();

        // Delegation expired — alice votes for herself
        assert_eq!(reg.resolve_delegate("alice", &scope()), None);
        assert!(reg.active_delegations(&scope()).is_empty());
    }

    #[test]
    fn active_delegations_filters_correctly() {
        let mut reg = DelegationRegistry::new();
        reg.delegate("alice", "bob", scope(), None).unwrap();
        let id = reg.delegate("carol", "dave", scope(), None).unwrap();
        reg.revoke(&id, "carol").unwrap();

        let active = reg.active_delegations(&scope());
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].from_did, "alice");
    }

    #[test]
    fn delegations_from_and_to() {
        let mut reg = DelegationRegistry::new();
        reg.delegate("alice", "bob", scope(), None).unwrap();
        reg.delegate("alice", "carol", prop_scope(), None).unwrap();

        assert_eq!(reg.delegations_from("alice").len(), 2);
        assert_eq!(reg.delegations_to("bob").len(), 1);
        assert_eq!(reg.delegations_to("carol").len(), 1);
    }

    #[test]
    fn registry_serializes() {
        let mut reg = DelegationRegistry::new();
        reg.delegate("alice", "bob", scope(), None).unwrap();
        reg.delegate("carol", "dave", scope(), None).unwrap();

        let json = serde_json::to_string(&reg).unwrap();
        let mut deserialized: DelegationRegistry = serde_json::from_str(&json).unwrap();
        deserialized.rebuild_index();
        assert_eq!(deserialized.total_delegations(), 2);
        assert_eq!(
            deserialized.resolve_delegate("alice", &scope()),
            Some("bob".into())
        );
    }

    #[test]
    fn revoked_not_in_effective_power() {
        let mut reg = DelegationRegistry::new();
        let id = reg.delegate("alice", "bob", scope(), None).unwrap();
        reg.revoke(&id, "alice").unwrap();

        let mut credits = HashMap::new();
        credits.insert("alice".to_string(), 100u64);
        credits.insert("bob".to_string(), 50u64);

        let power = reg.effective_power(&credits, &scope());
        // Revoked — alice keeps her own power
        assert_eq!(power.get("alice"), Some(&100));
        assert_eq!(power.get("bob"), Some(&50));
    }

    #[test]
    fn zero_credits_excluded() {
        let reg = DelegationRegistry::new();
        let mut credits = HashMap::new();
        credits.insert("alice".to_string(), 0u64);
        credits.insert("bob".to_string(), 50u64);

        let power = reg.effective_power(&credits, &scope());
        assert!(power.get("alice").is_none());
        assert_eq!(power.get("bob"), Some(&50));
    }
}
