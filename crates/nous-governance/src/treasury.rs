use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpendingStatus {
    Proposed,
    Approved,
    Executed,
    Rejected,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingProposal {
    pub id: String,
    pub dao_id: String,
    pub proposer: String,
    pub recipient: String,
    pub token: String,
    pub amount: u128,
    pub description: String,
    pub status: SpendingStatus,
    pub proposal_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub executed_at: Option<DateTime<Utc>>,
}

impl SpendingProposal {
    pub fn new(
        dao_id: &str,
        proposer: &str,
        recipient: &str,
        token: &str,
        amount: u128,
        description: &str,
    ) -> Result<Self> {
        if amount == 0 {
            return Err(Error::InvalidInput("amount must be positive".into()));
        }
        if proposer == recipient {
            return Err(Error::InvalidInput(
                "proposer and recipient must differ".into(),
            ));
        }

        Ok(Self {
            id: format!("spend:{}", Uuid::new_v4()),
            dao_id: dao_id.into(),
            proposer: proposer.into(),
            recipient: recipient.into(),
            token: token.into(),
            amount,
            description: description.into(),
            status: SpendingStatus::Proposed,
            proposal_id: None,
            created_at: Utc::now(),
            executed_at: None,
        })
    }

    pub fn link_proposal(&mut self, proposal_id: &str) {
        self.proposal_id = Some(proposal_id.into());
    }

    pub fn approve(&mut self) -> Result<()> {
        if self.status != SpendingStatus::Proposed {
            return Err(Error::InvalidInput(
                "spending is not in proposed state".into(),
            ));
        }
        self.status = SpendingStatus::Approved;
        Ok(())
    }

    pub fn execute(&mut self) -> Result<()> {
        if self.status != SpendingStatus::Approved {
            return Err(Error::InvalidInput("spending is not approved".into()));
        }
        self.status = SpendingStatus::Executed;
        self.executed_at = Some(Utc::now());
        Ok(())
    }

    pub fn reject(&mut self) -> Result<()> {
        if self.status != SpendingStatus::Proposed {
            return Err(Error::InvalidInput(
                "can only reject proposed spending".into(),
            ));
        }
        self.status = SpendingStatus::Rejected;
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<()> {
        if self.status == SpendingStatus::Executed {
            return Err(Error::InvalidInput(
                "cannot cancel executed spending".into(),
            ));
        }
        self.status = SpendingStatus::Cancelled;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Treasury {
    pub dao_id: String,
    pub balances: HashMap<String, u128>,
    pub spending_limit: Option<u128>,
    pub total_spent: HashMap<String, u128>,
    pub proposals: Vec<SpendingProposal>,
}

impl Treasury {
    pub fn new(dao_id: impl Into<String>) -> Self {
        Self {
            dao_id: dao_id.into(),
            balances: HashMap::new(),
            spending_limit: None,
            total_spent: HashMap::new(),
            proposals: Vec::new(),
        }
    }

    pub fn with_spending_limit(mut self, limit: u128) -> Self {
        self.spending_limit = Some(limit);
        self
    }

    pub fn deposit(&mut self, token: &str, amount: u128) {
        *self.balances.entry(token.to_string()).or_insert(0) += amount;
    }

    pub fn balance(&self, token: &str) -> u128 {
        self.balances.get(token).copied().unwrap_or(0)
    }

    pub fn total_balance(&self) -> u128 {
        self.balances.values().sum()
    }

    pub fn submit_spending(&mut self, proposal: SpendingProposal) -> Result<()> {
        if proposal.dao_id != self.dao_id {
            return Err(Error::InvalidInput(
                "proposal is for a different DAO".into(),
            ));
        }

        let balance = self.balance(&proposal.token);
        if proposal.amount > balance {
            return Err(Error::InvalidInput(format!(
                "insufficient treasury balance: have {balance}, need {}",
                proposal.amount
            )));
        }

        if let Some(limit) = self.spending_limit
            && proposal.amount > limit
        {
            return Err(Error::InvalidInput(format!(
                "amount {} exceeds spending limit {limit}",
                proposal.amount
            )));
        }

        self.proposals.push(proposal);
        Ok(())
    }

    pub fn execute_spending(&mut self, proposal_id: &str) -> Result<()> {
        let proposal = self
            .proposals
            .iter_mut()
            .find(|p| p.id == proposal_id)
            .ok_or_else(|| Error::NotFound("spending proposal not found".into()))?;

        if proposal.status != SpendingStatus::Approved {
            return Err(Error::InvalidInput("spending is not approved".into()));
        }

        let balance = self.balances.get(&proposal.token).copied().unwrap_or(0);
        if proposal.amount > balance {
            return Err(Error::InvalidInput(
                "insufficient balance at execution time".into(),
            ));
        }

        *self.balances.entry(proposal.token.clone()).or_insert(0) -= proposal.amount;
        *self.total_spent.entry(proposal.token.clone()).or_insert(0) += proposal.amount;

        proposal.status = SpendingStatus::Executed;
        proposal.executed_at = Some(Utc::now());
        Ok(())
    }

    pub fn pending_proposals(&self) -> Vec<&SpendingProposal> {
        self.proposals
            .iter()
            .filter(|p| p.status == SpendingStatus::Proposed)
            .collect()
    }

    pub fn approved_proposals(&self) -> Vec<&SpendingProposal> {
        self.proposals
            .iter()
            .filter(|p| p.status == SpendingStatus::Approved)
            .collect()
    }

    pub fn total_spent_for(&self, token: &str) -> u128 {
        self.total_spent.get(token).copied().unwrap_or(0)
    }

    pub fn spending_history(&self) -> Vec<&SpendingProposal> {
        self.proposals
            .iter()
            .filter(|p| p.status == SpendingStatus::Executed)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_treasury() {
        let treasury = Treasury::new("dao-1");
        assert_eq!(treasury.total_balance(), 0);
        assert!(treasury.pending_proposals().is_empty());
    }

    #[test]
    fn deposit_and_balance() {
        let mut treasury = Treasury::new("dao-1");
        treasury.deposit("ETH", 1000);
        treasury.deposit("NOUS", 5000);

        assert_eq!(treasury.balance("ETH"), 1000);
        assert_eq!(treasury.balance("NOUS"), 5000);
        assert_eq!(treasury.total_balance(), 6000);
    }

    #[test]
    fn multiple_deposits() {
        let mut treasury = Treasury::new("dao-1");
        treasury.deposit("ETH", 500);
        treasury.deposit("ETH", 300);
        assert_eq!(treasury.balance("ETH"), 800);
    }

    #[test]
    fn create_spending_proposal() {
        let proposal =
            SpendingProposal::new("dao-1", "alice", "bob", "ETH", 100, "Fund development").unwrap();
        assert_eq!(proposal.status, SpendingStatus::Proposed);
        assert!(proposal.id.starts_with("spend:"));
    }

    #[test]
    fn reject_zero_amount() {
        assert!(SpendingProposal::new("dao-1", "alice", "bob", "ETH", 0, "test").is_err());
    }

    #[test]
    fn reject_self_spending() {
        assert!(SpendingProposal::new("dao-1", "alice", "alice", "ETH", 100, "test").is_err());
    }

    #[test]
    fn spending_lifecycle() {
        let mut proposal =
            SpendingProposal::new("dao-1", "alice", "bob", "ETH", 100, "test").unwrap();

        proposal.approve().unwrap();
        assert_eq!(proposal.status, SpendingStatus::Approved);

        proposal.execute().unwrap();
        assert_eq!(proposal.status, SpendingStatus::Executed);
        assert!(proposal.executed_at.is_some());
    }

    #[test]
    fn reject_spending() {
        let mut proposal =
            SpendingProposal::new("dao-1", "alice", "bob", "ETH", 100, "test").unwrap();

        proposal.reject().unwrap();
        assert_eq!(proposal.status, SpendingStatus::Rejected);
    }

    #[test]
    fn cancel_spending() {
        let mut proposal =
            SpendingProposal::new("dao-1", "alice", "bob", "ETH", 100, "test").unwrap();

        proposal.cancel().unwrap();
        assert_eq!(proposal.status, SpendingStatus::Cancelled);
    }

    #[test]
    fn cannot_cancel_executed() {
        let mut proposal =
            SpendingProposal::new("dao-1", "alice", "bob", "ETH", 100, "test").unwrap();
        proposal.approve().unwrap();
        proposal.execute().unwrap();
        assert!(proposal.cancel().is_err());
    }

    #[test]
    fn treasury_submit_spending() {
        let mut treasury = Treasury::new("dao-1");
        treasury.deposit("ETH", 1000);

        let proposal = SpendingProposal::new("dao-1", "alice", "bob", "ETH", 500, "grant").unwrap();

        treasury.submit_spending(proposal).unwrap();
        assert_eq!(treasury.pending_proposals().len(), 1);
    }

    #[test]
    fn treasury_reject_insufficient_balance() {
        let mut treasury = Treasury::new("dao-1");
        treasury.deposit("ETH", 100);

        let proposal =
            SpendingProposal::new("dao-1", "alice", "bob", "ETH", 500, "too much").unwrap();

        assert!(treasury.submit_spending(proposal).is_err());
    }

    #[test]
    fn treasury_spending_limit() {
        let mut treasury = Treasury::new("dao-1").with_spending_limit(200);
        treasury.deposit("ETH", 1000);

        let proposal =
            SpendingProposal::new("dao-1", "alice", "bob", "ETH", 500, "over limit").unwrap();

        assert!(treasury.submit_spending(proposal).is_err());

        let small =
            SpendingProposal::new("dao-1", "alice", "bob", "ETH", 100, "under limit").unwrap();

        treasury.submit_spending(small).unwrap();
    }

    #[test]
    fn treasury_wrong_dao_rejected() {
        let mut treasury = Treasury::new("dao-1");
        treasury.deposit("ETH", 1000);

        let proposal =
            SpendingProposal::new("dao-2", "alice", "bob", "ETH", 100, "wrong dao").unwrap();

        assert!(treasury.submit_spending(proposal).is_err());
    }

    #[test]
    fn treasury_execute_spending() {
        let mut treasury = Treasury::new("dao-1");
        treasury.deposit("ETH", 1000);

        let mut proposal =
            SpendingProposal::new("dao-1", "alice", "bob", "ETH", 300, "grant").unwrap();
        proposal.approve().unwrap();
        let pid = proposal.id.clone();
        treasury.proposals.push(proposal);

        treasury.execute_spending(&pid).unwrap();

        assert_eq!(treasury.balance("ETH"), 700);
        assert_eq!(treasury.total_spent_for("ETH"), 300);
        assert_eq!(treasury.spending_history().len(), 1);
    }

    #[test]
    fn treasury_execute_unapproved_fails() {
        let mut treasury = Treasury::new("dao-1");
        treasury.deposit("ETH", 1000);

        let proposal = SpendingProposal::new("dao-1", "alice", "bob", "ETH", 100, "test").unwrap();
        let pid = proposal.id.clone();
        treasury.proposals.push(proposal);

        assert!(treasury.execute_spending(&pid).is_err());
    }

    #[test]
    fn link_proposal() {
        let mut proposal =
            SpendingProposal::new("dao-1", "alice", "bob", "ETH", 100, "test").unwrap();
        proposal.link_proposal("prop-123");
        assert_eq!(proposal.proposal_id.as_deref(), Some("prop-123"));
    }

    #[test]
    fn spending_proposal_serializes() {
        let proposal =
            SpendingProposal::new("dao-1", "alice", "bob", "ETH", 1000, "Development fund")
                .unwrap();
        let json = serde_json::to_string(&proposal).unwrap();
        let restored: SpendingProposal = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.amount, 1000);
    }

    #[test]
    fn treasury_serializes() {
        let mut treasury = Treasury::new("dao-1");
        treasury.deposit("ETH", 5000);
        let json = serde_json::to_string(&treasury).unwrap();
        let restored: Treasury = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.balance("ETH"), 5000);
    }
}
