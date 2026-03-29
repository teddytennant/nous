use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use nous_core::{Error, Result};

use crate::proposal::ProposalStatus;

/// The type of action a proposal executes when it passes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalAction {
    /// Transfer tokens from the DAO treasury to a recipient.
    TreasuryTransfer {
        recipient_did: String,
        token: String,
        amount: u64,
    },
    /// Update a DAO parameter (quorum, threshold, default_credits).
    ParameterChange { parameter: String, value: String },
    /// Add a member to the DAO.
    AddMember { did: String },
    /// Remove a member from the DAO.
    RemoveMember { did: String },
    /// Grant credits to a member.
    GrantCredits { did: String, amount: u64 },
    /// Arbitrary on-chain call (contract address + calldata).
    ExternalCall { target: String, calldata: Vec<u8> },
}

/// Status of a queued execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Waiting for the timelock to expire.
    Queued,
    /// Timelock expired, ready to execute.
    Ready,
    /// Successfully executed.
    Executed,
    /// Execution failed (action returned an error).
    Failed,
    /// Cancelled before execution (e.g. by veto or governance override).
    Cancelled,
}

/// A queued execution: a passed proposal waiting for its timelock to expire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedExecution {
    pub id: String,
    pub proposal_id: String,
    pub dao_id: String,
    pub actions: Vec<ProposalAction>,
    pub queued_at: DateTime<Utc>,
    pub executable_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: ExecutionStatus,
    pub executed_at: Option<DateTime<Utc>>,
    pub executor_did: Option<String>,
    pub error: Option<String>,
}

impl QueuedExecution {
    pub fn is_executable(&self) -> bool {
        let now = Utc::now();
        self.status == ExecutionStatus::Queued && now >= self.executable_at && now < self.expires_at
    }

    pub fn is_expired(&self) -> bool {
        self.status == ExecutionStatus::Queued && Utc::now() >= self.expires_at
    }
}

/// Result of executing a single action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action: ProposalAction,
    pub success: bool,
    pub message: String,
}

/// The execution engine: manages timelock queue, validates, and executes proposal actions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionEngine {
    /// Queued executions indexed by ID.
    queue: HashMap<String, QueuedExecution>,
    /// Index: proposal_id → execution_id.
    by_proposal: HashMap<String, String>,
    /// Default timelock duration (how long to wait after a proposal passes).
    pub timelock_duration: i64,
    /// Grace period: how long after the timelock an execution remains valid.
    pub grace_period: i64,
}

impl ExecutionEngine {
    /// Create a new execution engine with the given timelock (in seconds).
    pub fn new(timelock_seconds: i64, grace_period_seconds: i64) -> Self {
        Self {
            queue: HashMap::new(),
            by_proposal: HashMap::new(),
            timelock_duration: timelock_seconds,
            grace_period: grace_period_seconds,
        }
    }

    /// Queue a passed proposal for execution.
    /// The proposal must be in Passed status and not already queued.
    pub fn queue_proposal(
        &mut self,
        proposal_id: &str,
        dao_id: &str,
        proposal_status: ProposalStatus,
        actions: Vec<ProposalAction>,
    ) -> Result<String> {
        if proposal_status != ProposalStatus::Passed {
            return Err(Error::Governance(
                "only passed proposals can be queued for execution".into(),
            ));
        }

        if actions.is_empty() {
            return Err(Error::Governance(
                "proposal must have at least one action".into(),
            ));
        }

        if self.by_proposal.contains_key(proposal_id) {
            return Err(Error::Governance(
                "proposal is already queued for execution".into(),
            ));
        }

        let now = Utc::now();
        let id = format!("exec:{}", Uuid::new_v4());
        let execution = QueuedExecution {
            id: id.clone(),
            proposal_id: proposal_id.to_string(),
            dao_id: dao_id.to_string(),
            actions,
            queued_at: now,
            executable_at: now + Duration::seconds(self.timelock_duration),
            expires_at: now + Duration::seconds(self.timelock_duration + self.grace_period),
            status: ExecutionStatus::Queued,
            executed_at: None,
            executor_did: None,
            error: None,
        };

        self.queue.insert(id.clone(), execution);
        self.by_proposal.insert(proposal_id.to_string(), id.clone());
        Ok(id)
    }

    /// Execute a queued proposal. Returns action results.
    /// The executor_did is recorded for audit purposes.
    /// Actions are validated but not actually applied — the caller is responsible
    /// for applying the ActionResults to DAO state.
    pub fn execute(&mut self, execution_id: &str, executor_did: &str) -> Result<Vec<ActionResult>> {
        let execution = self
            .queue
            .get(execution_id)
            .ok_or_else(|| Error::NotFound("execution not found".into()))?;

        if execution.status != ExecutionStatus::Queued {
            return Err(Error::Governance(format!(
                "execution is {:?}, not Queued",
                execution.status
            )));
        }

        let now = Utc::now();

        if now < execution.executable_at {
            let remaining = execution.executable_at - now;
            return Err(Error::Governance(format!(
                "timelock has not expired; {} seconds remaining",
                remaining.num_seconds()
            )));
        }

        if now >= execution.expires_at {
            let execution = self.queue.get_mut(execution_id).unwrap();
            execution.status = ExecutionStatus::Failed;
            execution.error = Some("execution window expired".into());
            return Err(Error::Expired("execution window has expired".into()));
        }

        // Validate and produce results for each action
        let results: Vec<ActionResult> = execution.actions.iter().map(validate_action).collect();

        let all_success = results.iter().all(|r| r.success);

        let execution = self.queue.get_mut(execution_id).unwrap();
        execution.executed_at = Some(now);
        execution.executor_did = Some(executor_did.to_string());

        if all_success {
            execution.status = ExecutionStatus::Executed;
        } else {
            execution.status = ExecutionStatus::Failed;
            let errors: Vec<&str> = results
                .iter()
                .filter(|r| !r.success)
                .map(|r| r.message.as_str())
                .collect();
            execution.error = Some(errors.join("; "));
        }

        Ok(results)
    }

    /// Cancel a queued execution before it runs.
    pub fn cancel(&mut self, execution_id: &str) -> Result<()> {
        let execution = self
            .queue
            .get_mut(execution_id)
            .ok_or_else(|| Error::NotFound("execution not found".into()))?;

        if execution.status != ExecutionStatus::Queued {
            return Err(Error::Governance(
                "can only cancel queued executions".into(),
            ));
        }

        execution.status = ExecutionStatus::Cancelled;
        Ok(())
    }

    /// Get a queued execution by ID.
    pub fn get(&self, execution_id: &str) -> Option<&QueuedExecution> {
        self.queue.get(execution_id)
    }

    /// Get execution by proposal ID.
    pub fn get_by_proposal(&self, proposal_id: &str) -> Option<&QueuedExecution> {
        self.by_proposal
            .get(proposal_id)
            .and_then(|id| self.queue.get(id))
    }

    /// List all queued executions for a given DAO.
    pub fn list_for_dao(&self, dao_id: &str) -> Vec<&QueuedExecution> {
        self.queue.values().filter(|e| e.dao_id == dao_id).collect()
    }

    /// List all executions that are currently ready to execute.
    pub fn ready_executions(&self) -> Vec<&QueuedExecution> {
        self.queue.values().filter(|e| e.is_executable()).collect()
    }

    /// Total number of executions.
    pub fn total(&self) -> usize {
        self.queue.len()
    }
}

/// Validate a single action. Returns success/failure with a message.
fn validate_action(action: &ProposalAction) -> ActionResult {
    match action {
        ProposalAction::TreasuryTransfer {
            recipient_did,
            token,
            amount,
        } => {
            if recipient_did.is_empty() {
                return ActionResult {
                    action: action.clone(),
                    success: false,
                    message: "recipient DID is empty".into(),
                };
            }
            if token.is_empty() {
                return ActionResult {
                    action: action.clone(),
                    success: false,
                    message: "token is empty".into(),
                };
            }
            if *amount == 0 {
                return ActionResult {
                    action: action.clone(),
                    success: false,
                    message: "transfer amount is zero".into(),
                };
            }
            ActionResult {
                action: action.clone(),
                success: true,
                message: format!("transfer {} {} to {}", amount, token, recipient_did),
            }
        }
        ProposalAction::ParameterChange { parameter, value } => {
            let valid_params = ["quorum", "threshold", "default_credits", "timelock"];
            if !valid_params.contains(&parameter.as_str()) {
                return ActionResult {
                    action: action.clone(),
                    success: false,
                    message: format!("unknown parameter: {}", parameter),
                };
            }
            ActionResult {
                action: action.clone(),
                success: true,
                message: format!("set {} = {}", parameter, value),
            }
        }
        ProposalAction::AddMember { did } => {
            if did.is_empty() {
                return ActionResult {
                    action: action.clone(),
                    success: false,
                    message: "member DID is empty".into(),
                };
            }
            ActionResult {
                action: action.clone(),
                success: true,
                message: format!("add member {}", did),
            }
        }
        ProposalAction::RemoveMember { did } => {
            if did.is_empty() {
                return ActionResult {
                    action: action.clone(),
                    success: false,
                    message: "member DID is empty".into(),
                };
            }
            ActionResult {
                action: action.clone(),
                success: true,
                message: format!("remove member {}", did),
            }
        }
        ProposalAction::GrantCredits { did, amount } => {
            if did.is_empty() {
                return ActionResult {
                    action: action.clone(),
                    success: false,
                    message: "member DID is empty".into(),
                };
            }
            if *amount == 0 {
                return ActionResult {
                    action: action.clone(),
                    success: false,
                    message: "credit amount is zero".into(),
                };
            }
            ActionResult {
                action: action.clone(),
                success: true,
                message: format!("grant {} credits to {}", amount, did),
            }
        }
        ProposalAction::ExternalCall { target, calldata } => {
            if target.is_empty() {
                return ActionResult {
                    action: action.clone(),
                    success: false,
                    message: "call target is empty".into(),
                };
            }
            ActionResult {
                action: action.clone(),
                success: true,
                message: format!("call {} with {} bytes", target, calldata.len()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> ExecutionEngine {
        // 0 second timelock for testing (immediately executable), 1 hour grace period
        ExecutionEngine::new(0, 3600)
    }

    fn engine_with_timelock() -> ExecutionEngine {
        // 1 hour timelock, 1 hour grace period
        ExecutionEngine::new(3600, 3600)
    }

    fn transfer_action() -> ProposalAction {
        ProposalAction::TreasuryTransfer {
            recipient_did: "did:key:zrecipient".into(),
            token: "NOUS".into(),
            amount: 1000,
        }
    }

    #[test]
    fn queue_passed_proposal() {
        let mut engine = engine();
        let id = engine
            .queue_proposal(
                "prop-1",
                "dao-1",
                ProposalStatus::Passed,
                vec![transfer_action()],
            )
            .unwrap();

        assert!(id.starts_with("exec:"));
        let exec = engine.get(&id).unwrap();
        assert_eq!(exec.status, ExecutionStatus::Queued);
        assert_eq!(exec.proposal_id, "prop-1");
        assert_eq!(exec.dao_id, "dao-1");
    }

    #[test]
    fn reject_non_passed_proposal() {
        let mut engine = engine();

        for status in [
            ProposalStatus::Draft,
            ProposalStatus::Active,
            ProposalStatus::Rejected,
            ProposalStatus::Cancelled,
            ProposalStatus::Executed,
        ] {
            let result = engine.queue_proposal("prop-x", "dao-1", status, vec![transfer_action()]);
            assert!(result.is_err());
        }
    }

    #[test]
    fn reject_empty_actions() {
        let mut engine = engine();
        let result = engine.queue_proposal("prop-1", "dao-1", ProposalStatus::Passed, vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn reject_duplicate_queue() {
        let mut engine = engine();
        engine
            .queue_proposal(
                "prop-1",
                "dao-1",
                ProposalStatus::Passed,
                vec![transfer_action()],
            )
            .unwrap();
        let result = engine.queue_proposal(
            "prop-1",
            "dao-1",
            ProposalStatus::Passed,
            vec![transfer_action()],
        );
        assert!(result.is_err());
    }

    #[test]
    fn execute_immediately_with_zero_timelock() {
        let mut engine = engine();
        let id = engine
            .queue_proposal(
                "prop-1",
                "dao-1",
                ProposalStatus::Passed,
                vec![transfer_action()],
            )
            .unwrap();

        let results = engine.execute(&id, "did:key:zexecutor").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].success);

        let exec = engine.get(&id).unwrap();
        assert_eq!(exec.status, ExecutionStatus::Executed);
        assert!(exec.executed_at.is_some());
        assert_eq!(exec.executor_did.as_deref(), Some("did:key:zexecutor"));
    }

    #[test]
    fn timelock_blocks_early_execution() {
        let mut engine = engine_with_timelock();
        let id = engine
            .queue_proposal(
                "prop-1",
                "dao-1",
                ProposalStatus::Passed,
                vec![transfer_action()],
            )
            .unwrap();

        let result = engine.execute(&id, "did:key:zexecutor");
        assert!(result.is_err());

        let exec = engine.get(&id).unwrap();
        assert_eq!(exec.status, ExecutionStatus::Queued); // Still queued
    }

    #[test]
    fn cancel_queued_execution() {
        let mut engine = engine();
        let id = engine
            .queue_proposal(
                "prop-1",
                "dao-1",
                ProposalStatus::Passed,
                vec![transfer_action()],
            )
            .unwrap();

        engine.cancel(&id).unwrap();
        let exec = engine.get(&id).unwrap();
        assert_eq!(exec.status, ExecutionStatus::Cancelled);
    }

    #[test]
    fn cannot_cancel_executed() {
        let mut engine = engine();
        let id = engine
            .queue_proposal(
                "prop-1",
                "dao-1",
                ProposalStatus::Passed,
                vec![transfer_action()],
            )
            .unwrap();
        engine.execute(&id, "executor").unwrap();

        let result = engine.cancel(&id);
        assert!(result.is_err());
    }

    #[test]
    fn cannot_execute_cancelled() {
        let mut engine = engine();
        let id = engine
            .queue_proposal(
                "prop-1",
                "dao-1",
                ProposalStatus::Passed,
                vec![transfer_action()],
            )
            .unwrap();
        engine.cancel(&id).unwrap();

        let result = engine.execute(&id, "executor");
        assert!(result.is_err());
    }

    #[test]
    fn multiple_actions_all_pass() {
        let mut engine = engine();
        let actions = vec![
            transfer_action(),
            ProposalAction::AddMember {
                did: "did:key:znew".into(),
            },
            ProposalAction::ParameterChange {
                parameter: "quorum".into(),
                value: "0.2".into(),
            },
        ];

        let id = engine
            .queue_proposal("prop-1", "dao-1", ProposalStatus::Passed, actions)
            .unwrap();
        let results = engine.execute(&id, "executor").unwrap();
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.success));
    }

    #[test]
    fn action_failure_marks_execution_failed() {
        let mut engine = engine();
        let actions = vec![
            transfer_action(),
            ProposalAction::TreasuryTransfer {
                recipient_did: "".into(), // Empty DID — will fail validation
                token: "NOUS".into(),
                amount: 100,
            },
        ];

        let id = engine
            .queue_proposal("prop-1", "dao-1", ProposalStatus::Passed, actions)
            .unwrap();
        let results = engine.execute(&id, "executor").unwrap();
        assert!(results[0].success);
        assert!(!results[1].success);

        let exec = engine.get(&id).unwrap();
        assert_eq!(exec.status, ExecutionStatus::Failed);
        assert!(exec.error.is_some());
    }

    #[test]
    fn validate_transfer_zero_amount() {
        let result = validate_action(&ProposalAction::TreasuryTransfer {
            recipient_did: "did:key:z".into(),
            token: "ETH".into(),
            amount: 0,
        });
        assert!(!result.success);
    }

    #[test]
    fn validate_unknown_parameter() {
        let result = validate_action(&ProposalAction::ParameterChange {
            parameter: "nonexistent".into(),
            value: "42".into(),
        });
        assert!(!result.success);
    }

    #[test]
    fn validate_grant_credits_zero() {
        let result = validate_action(&ProposalAction::GrantCredits {
            did: "did:key:z".into(),
            amount: 0,
        });
        assert!(!result.success);
    }

    #[test]
    fn validate_external_call() {
        let result = validate_action(&ProposalAction::ExternalCall {
            target: "0xdeadbeef".into(),
            calldata: vec![0x01, 0x02],
        });
        assert!(result.success);
    }

    #[test]
    fn validate_external_call_empty_target() {
        let result = validate_action(&ProposalAction::ExternalCall {
            target: "".into(),
            calldata: vec![],
        });
        assert!(!result.success);
    }

    #[test]
    fn get_by_proposal() {
        let mut engine = engine();
        engine
            .queue_proposal(
                "prop-1",
                "dao-1",
                ProposalStatus::Passed,
                vec![transfer_action()],
            )
            .unwrap();

        let exec = engine.get_by_proposal("prop-1").unwrap();
        assert_eq!(exec.proposal_id, "prop-1");
        assert!(engine.get_by_proposal("prop-999").is_none());
    }

    #[test]
    fn list_for_dao() {
        let mut engine = engine();
        engine
            .queue_proposal(
                "prop-1",
                "dao-1",
                ProposalStatus::Passed,
                vec![transfer_action()],
            )
            .unwrap();
        engine
            .queue_proposal(
                "prop-2",
                "dao-1",
                ProposalStatus::Passed,
                vec![transfer_action()],
            )
            .unwrap();
        engine
            .queue_proposal(
                "prop-3",
                "dao-2",
                ProposalStatus::Passed,
                vec![transfer_action()],
            )
            .unwrap();

        assert_eq!(engine.list_for_dao("dao-1").len(), 2);
        assert_eq!(engine.list_for_dao("dao-2").len(), 1);
    }

    #[test]
    fn ready_executions_with_zero_timelock() {
        let mut engine = engine();
        engine
            .queue_proposal(
                "prop-1",
                "dao-1",
                ProposalStatus::Passed,
                vec![transfer_action()],
            )
            .unwrap();
        engine
            .queue_proposal(
                "prop-2",
                "dao-1",
                ProposalStatus::Passed,
                vec![transfer_action()],
            )
            .unwrap();

        assert_eq!(engine.ready_executions().len(), 2);
    }

    #[test]
    fn ready_executions_with_timelock() {
        let mut engine = engine_with_timelock();
        engine
            .queue_proposal(
                "prop-1",
                "dao-1",
                ProposalStatus::Passed,
                vec![transfer_action()],
            )
            .unwrap();

        // Not ready yet — timelock hasn't expired
        assert!(engine.ready_executions().is_empty());
    }

    #[test]
    fn engine_serializes() {
        let mut engine = engine();
        engine
            .queue_proposal(
                "prop-1",
                "dao-1",
                ProposalStatus::Passed,
                vec![transfer_action()],
            )
            .unwrap();

        let json = serde_json::to_string(&engine).unwrap();
        let deserialized: ExecutionEngine = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total(), 1);
        assert!(deserialized.get_by_proposal("prop-1").is_some());
    }
}
