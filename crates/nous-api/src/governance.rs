use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use nous_governance::{
    Ballot, CommittedVote, Dao, DelegationScope, Proposal, ProposalAction, VoteChoice, VoteResult,
    VoteTally,
};

use crate::error::ApiError;
use crate::state::AppState;

// ── DAO Types ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDaoRequest {
    pub founder_did: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DaoResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub founder_did: String,
    pub member_count: usize,
    pub created_at: String,
}

impl From<&Dao> for DaoResponse {
    fn from(dao: &Dao) -> Self {
        Self {
            id: dao.id.clone(),
            name: dao.name.clone(),
            description: dao.description.clone(),
            founder_did: dao.founder_did.clone(),
            member_count: dao.member_count(),
            created_at: dao.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DaoListResponse {
    pub daos: Vec<DaoResponse>,
    pub count: usize,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddMemberRequest {
    pub did: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemberResponse {
    pub did: String,
    pub credits: u64,
    pub role: String,
    pub joined_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DaoDetailResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub founder_did: String,
    pub member_count: usize,
    pub created_at: String,
    pub members: Vec<MemberResponse>,
    pub default_quorum: f64,
    pub default_threshold: f64,
}

impl From<&Dao> for DaoDetailResponse {
    fn from(dao: &Dao) -> Self {
        let members = dao
            .members
            .values()
            .map(|m| MemberResponse {
                did: m.did.clone(),
                credits: m.credits,
                role: format!("{:?}", m.role),
                joined_at: m.joined_at.to_rfc3339(),
            })
            .collect();

        Self {
            id: dao.id.clone(),
            name: dao.name.clone(),
            description: dao.description.clone(),
            founder_did: dao.founder_did.clone(),
            member_count: dao.member_count(),
            created_at: dao.created_at.to_rfc3339(),
            members,
            default_quorum: dao.default_quorum,
            default_threshold: dao.default_threshold,
        }
    }
}

// ── Proposal Types ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct ProposalResponse {
    pub id: String,
    pub dao_id: String,
    pub title: String,
    pub description: String,
    pub proposer_did: String,
    pub status: String,
    pub created_at: String,
    pub voting_starts: String,
    pub voting_ends: String,
    pub quorum: f64,
    pub threshold: f64,
}

impl From<&Proposal> for ProposalResponse {
    fn from(p: &Proposal) -> Self {
        Self {
            id: p.id.clone(),
            dao_id: p.dao_id.clone(),
            title: p.title.clone(),
            description: p.description.clone(),
            proposer_did: p.proposer_did.clone(),
            status: format!("{:?}", p.status),
            created_at: p.created_at.to_rfc3339(),
            voting_starts: p.voting_starts.to_rfc3339(),
            voting_ends: p.voting_ends.to_rfc3339(),
            quorum: p.quorum,
            threshold: p.threshold,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProposalListResponse {
    pub proposals: Vec<ProposalResponse>,
    pub count: usize,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ProposalQuery {
    pub dao_id: Option<String>,
    pub status: Option<String>,
}

// ── Vote Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct VoteResultResponse {
    pub proposal_id: String,
    pub votes_for: u64,
    pub votes_against: u64,
    pub votes_abstain: u64,
    pub total_voters: usize,
    pub passed: bool,
}

impl From<VoteResult> for VoteResultResponse {
    fn from(r: VoteResult) -> Self {
        Self {
            proposal_id: r.proposal_id,
            votes_for: r.votes_for,
            votes_against: r.votes_against,
            votes_abstain: r.votes_abstain,
            total_voters: r.total_voters,
            passed: r.passed,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrivateTallyResponse {
    pub proposal_id: String,
    pub total_votes: usize,
    pub votes_for: usize,
    pub votes_against: usize,
    pub votes_abstain: usize,
    pub all_proofs_valid: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MutationResponse {
    pub success: bool,
    pub message: String,
}

// ── DAO Handlers ───────────────────────────────────────────────────────────

#[utoipa::path(
    post, path = "/api/v1/daos",
    tag = "governance",
    request_body = CreateDaoRequest,
    responses(
        (status = 200, description = "DAO created"),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_dao(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateDaoRequest>,
) -> Result<Json<DaoResponse>, ApiError> {
    if req.name.is_empty() {
        return Err(ApiError::bad_request("name cannot be empty"));
    }
    if req.founder_did.is_empty() {
        return Err(ApiError::bad_request("founder_did cannot be empty"));
    }

    let dao = Dao::create(&req.founder_did, &req.name, &req.description);
    let response = DaoResponse::from(&dao);

    let id = dao.id.clone();
    let mut daos = state.daos.write().await;
    daos.insert(id.clone(), dao);

    // Persist DAO to SQLite
    if let Some(d) = daos.get(&id) {
        state.persist_dao(&id, d).await;
    }

    state.emit(crate::state::RealtimeEvent::DaoCreated {
        id: response.id.clone(),
        name: response.name.clone(),
    });

    Ok(Json(response))
}

#[utoipa::path(
    get, path = "/api/v1/daos",
    tag = "governance",
    responses((status = 200, description = "List of all DAOs"))
)]
pub async fn list_daos(State(state): State<Arc<AppState>>) -> Json<DaoListResponse> {
    let daos = state.daos.read().await;
    let dao_list: Vec<DaoResponse> = daos.values().map(DaoResponse::from).collect();
    let count = dao_list.len();
    Json(DaoListResponse {
        daos: dao_list,
        count,
    })
}

#[utoipa::path(
    get, path = "/api/v1/daos/{dao_id}",
    tag = "governance",
    params(("dao_id" = String, Path, description = "DAO identifier")),
    responses(
        (status = 200, description = "DAO details with members"),
        (status = 404, description = "DAO not found")
    )
)]
pub async fn get_dao(
    State(state): State<Arc<AppState>>,
    Path(dao_id): Path<String>,
) -> Result<Json<DaoDetailResponse>, ApiError> {
    let daos = state.daos.read().await;
    let dao = daos
        .get(&dao_id)
        .ok_or_else(|| ApiError::not_found(format!("DAO {dao_id} not found")))?;

    Ok(Json(DaoDetailResponse::from(dao)))
}

#[utoipa::path(
    post, path = "/api/v1/daos/{dao_id}/members",
    tag = "governance",
    params(("dao_id" = String, Path, description = "DAO identifier")),
    request_body = AddMemberRequest,
    responses(
        (status = 200, description = "Member added"),
        (status = 404, description = "DAO not found"),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn add_member(
    State(state): State<Arc<AppState>>,
    Path(dao_id): Path<String>,
    Json(req): Json<AddMemberRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    if req.did.is_empty() {
        return Err(ApiError::bad_request("did cannot be empty"));
    }

    let mut daos = state.daos.write().await;
    let dao = daos
        .get_mut(&dao_id)
        .ok_or_else(|| ApiError::not_found(format!("DAO {dao_id} not found")))?;

    dao.add_member(&req.did).map_err(ApiError::from)?;

    // Persist DAO to SQLite
    state.persist_dao(&dao_id, dao).await;

    Ok(Json(MutationResponse {
        success: true,
        message: format!("member {} added to {}", req.did, dao_id),
    }))
}

#[utoipa::path(
    delete, path = "/api/v1/daos/{dao_id}/members/{did}",
    tag = "governance",
    params(
        ("dao_id" = String, Path, description = "DAO identifier"),
        ("did" = String, Path, description = "Member DID to remove")
    ),
    responses(
        (status = 200, description = "Member removed"),
        (status = 404, description = "DAO or member not found"),
        (status = 401, description = "Cannot remove founder")
    )
)]
pub async fn remove_member(
    State(state): State<Arc<AppState>>,
    Path((dao_id, did)): Path<(String, String)>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut daos = state.daos.write().await;
    let dao = daos
        .get_mut(&dao_id)
        .ok_or_else(|| ApiError::not_found(format!("DAO {dao_id} not found")))?;

    dao.remove_member(&did).map_err(ApiError::from)?;

    // Persist DAO to SQLite
    state.persist_dao(&dao_id, dao).await;

    Ok(Json(MutationResponse {
        success: true,
        message: format!("member {} removed from {}", did, dao_id),
    }))
}

// ── Proposal Handlers ──────────────────────────────────────────────────────

#[utoipa::path(
    post, path = "/api/v1/proposals",
    tag = "governance",
    request_body = String,
    responses(
        (status = 200, description = "Proposal submitted"),
        (status = 400, description = "Invalid or unverifiable proposal")
    )
)]
pub async fn submit_proposal(
    State(state): State<Arc<AppState>>,
    Json(proposal): Json<Proposal>,
) -> Result<Json<ProposalResponse>, ApiError> {
    proposal
        .verify()
        .map_err(|e| ApiError::bad_request(format!("invalid proposal signature: {e}")))?;

    // Verify proposer is a member of the DAO
    let daos = state.daos.read().await;
    if let Some(dao) = daos.get(&proposal.dao_id) {
        if !dao.is_member(&proposal.proposer_did) {
            return Err(ApiError::unauthorized("proposer is not a DAO member"));
        }
    } else {
        return Err(ApiError::not_found(format!(
            "DAO {} not found",
            proposal.dao_id
        )));
    }
    drop(daos);

    let response = ProposalResponse::from(&proposal);

    // Create a VoteTally for this proposal
    let tally = VoteTally::new(&proposal.id, proposal.quorum, proposal.threshold);

    let proposal_id = proposal.id.clone();
    let mut proposals = state.proposals.write().await;
    proposals.insert(proposal_id.clone(), proposal);

    // Persist proposal to SQLite
    if let Some(p) = proposals.get(&proposal_id) {
        state.persist_proposal(&proposal_id, p).await;
    }
    drop(proposals);

    let tally_id = response.id.clone();
    let mut tallies = state.tallies.write().await;
    tallies.insert(tally_id.clone(), tally);

    // Persist tally to SQLite
    if let Some(t) = tallies.get(&tally_id) {
        state.persist_tally(&tally_id, t).await;
    }

    state.emit(crate::state::RealtimeEvent::ProposalCreated {
        id: response.id.clone(),
        title: response.title.clone(),
        dao_id: response.dao_id.clone(),
    });

    Ok(Json(response))
}

#[utoipa::path(
    get, path = "/api/v1/proposals",
    tag = "governance",
    params(ProposalQuery),
    responses((status = 200, description = "List of proposals"))
)]
pub async fn list_proposals(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ProposalQuery>,
) -> Json<ProposalListResponse> {
    let proposals = state.proposals.read().await;

    let filtered: Vec<ProposalResponse> = proposals
        .values()
        .filter(|p| {
            if let Some(ref dao_id) = query.dao_id
                && &p.dao_id != dao_id
            {
                return false;
            }
            if let Some(ref status) = query.status
                && &format!("{:?}", p.status) != status
            {
                return false;
            }
            true
        })
        .map(ProposalResponse::from)
        .collect();

    let count = filtered.len();
    Json(ProposalListResponse {
        proposals: filtered,
        count,
    })
}

#[utoipa::path(
    get, path = "/api/v1/proposals/{proposal_id}",
    tag = "governance",
    params(("proposal_id" = String, Path, description = "Proposal identifier")),
    responses(
        (status = 200, description = "Proposal details"),
        (status = 404, description = "Proposal not found")
    )
)]
pub async fn get_proposal(
    State(state): State<Arc<AppState>>,
    Path(proposal_id): Path<String>,
) -> Result<Json<ProposalResponse>, ApiError> {
    let proposals = state.proposals.read().await;
    let proposal = proposals
        .get(&proposal_id)
        .ok_or_else(|| ApiError::not_found(format!("proposal {proposal_id} not found")))?;

    Ok(Json(ProposalResponse::from(proposal)))
}

// ── Voting Handlers ────────────────────────────────────────────────────────

#[utoipa::path(
    post, path = "/api/v1/votes",
    tag = "governance",
    request_body = String,
    responses(
        (status = 200, description = "Vote cast"),
        (status = 400, description = "Invalid ballot"),
        (status = 404, description = "Proposal not found")
    )
)]
pub async fn cast_vote(
    State(state): State<Arc<AppState>>,
    Json(ballot): Json<Ballot>,
) -> Result<Json<MutationResponse>, ApiError> {
    ballot
        .verify()
        .map_err(|e| ApiError::bad_request(format!("invalid ballot signature: {e}")))?;

    // Verify the proposal exists
    let proposals = state.proposals.read().await;
    if !proposals.contains_key(&ballot.proposal_id) {
        return Err(ApiError::not_found(format!(
            "proposal {} not found",
            ballot.proposal_id
        )));
    }
    drop(proposals);

    let mut tallies = state.tallies.write().await;
    let tally = tallies
        .get_mut(&ballot.proposal_id)
        .ok_or_else(|| ApiError::internal("tally not initialized for proposal"))?;

    let proposal_id = ballot.proposal_id.clone();
    let voter = ballot.voter_did.clone();
    tally.cast(ballot).map_err(ApiError::from)?;

    // Persist tally to SQLite
    state.persist_tally(&proposal_id, tally).await;

    state.emit(crate::state::RealtimeEvent::VoteCast { proposal_id, voter });

    Ok(Json(MutationResponse {
        success: true,
        message: "vote cast".into(),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/votes/{proposal_id}",
    tag = "governance",
    params(("proposal_id" = String, Path, description = "Proposal to tally")),
    responses(
        (status = 200, description = "Vote tally result"),
        (status = 404, description = "Proposal not found")
    )
)]
pub async fn get_tally(
    State(state): State<Arc<AppState>>,
    Path(proposal_id): Path<String>,
) -> Result<Json<VoteResultResponse>, ApiError> {
    // Get eligible voter count from DAO
    let proposals = state.proposals.read().await;
    let proposal = proposals
        .get(&proposal_id)
        .ok_or_else(|| ApiError::not_found(format!("proposal {proposal_id} not found")))?;
    let dao_id = proposal.dao_id.clone();
    drop(proposals);

    let daos = state.daos.read().await;
    let eligible_voters = daos.get(&dao_id).map(|d| d.member_count()).unwrap_or(0);
    drop(daos);

    let tallies = state.tallies.read().await;
    let tally = tallies
        .get(&proposal_id)
        .ok_or_else(|| ApiError::not_found("tally not found"))?;

    let result = tally.tally(eligible_voters);
    Ok(Json(VoteResultResponse::from(result)))
}

// ── ZK Private Voting Handlers ─────────────────────────────────────────────

#[utoipa::path(
    post, path = "/api/v1/votes/private",
    tag = "governance",
    request_body = String,
    responses(
        (status = 200, description = "Private vote submitted"),
        (status = 400, description = "Invalid proof"),
        (status = 404, description = "Proposal not found")
    )
)]
pub async fn cast_private_vote(
    State(state): State<Arc<AppState>>,
    Json(vote): Json<CommittedVote>,
) -> Result<Json<MutationResponse>, ApiError> {
    nous_governance::verify_committed_vote(&vote)
        .map_err(|e| ApiError::bad_request(format!("invalid ZK proof: {e}")))?;

    let proposals = state.proposals.read().await;
    if !proposals.contains_key(&vote.proposal_id) {
        return Err(ApiError::not_found(format!(
            "proposal {} not found",
            vote.proposal_id
        )));
    }
    drop(proposals);

    let proposal_id = vote.proposal_id.clone();
    let mut private_votes = state.private_votes.write().await;
    let votes = private_votes.entry(proposal_id.clone()).or_default();
    votes.push(vote);

    // Persist private votes to SQLite
    state.persist_private_votes(&proposal_id, votes).await;

    Ok(Json(MutationResponse {
        success: true,
        message: "private vote submitted".into(),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/votes/private/{proposal_id}",
    tag = "governance",
    params(("proposal_id" = String, Path, description = "Proposal to tally privately")),
    responses(
        (status = 200, description = "Private tally result"),
        (status = 404, description = "Proposal not found")
    )
)]
pub async fn get_private_tally(
    State(state): State<Arc<AppState>>,
    Path(proposal_id): Path<String>,
) -> Result<Json<PrivateTallyResponse>, ApiError> {
    let proposals = state.proposals.read().await;
    if !proposals.contains_key(&proposal_id) {
        return Err(ApiError::not_found(format!(
            "proposal {proposal_id} not found"
        )));
    }
    drop(proposals);

    let private_votes = state.private_votes.read().await;
    let votes = private_votes.get(&proposal_id);

    let empty = vec![];
    let votes_ref = votes.unwrap_or(&empty);

    let result =
        nous_governance::tally_private_votes(&proposal_id, votes_ref).map_err(ApiError::from)?;

    Ok(Json(PrivateTallyResponse {
        proposal_id: result.proposal_id,
        total_votes: result.total_votes,
        votes_for: result.votes_for,
        votes_against: result.votes_against,
        votes_abstain: result.votes_abstain,
        all_proofs_valid: result.all_proofs_valid,
    }))
}

// ── Convenience Handlers (custodial signing) ─────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct SimpleProposalRequest {
    pub proposer_did: String,
    pub title: String,
    pub description: String,
    pub quorum: Option<f64>,
    pub threshold: Option<f64>,
    pub voting_days: Option<i64>,
}

#[utoipa::path(
    post, path = "/api/v1/daos/{dao_id}/proposals",
    tag = "governance",
    params(("dao_id" = String, Path, description = "DAO identifier")),
    request_body = SimpleProposalRequest,
    responses(
        (status = 200, description = "Proposal created"),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "DAO or identity not found")
    )
)]
pub async fn create_proposal(
    State(state): State<Arc<AppState>>,
    Path(dao_id): Path<String>,
    Json(req): Json<SimpleProposalRequest>,
) -> Result<Json<ProposalResponse>, ApiError> {
    if req.title.is_empty() {
        return Err(ApiError::bad_request("title cannot be empty"));
    }
    if req.proposer_did.is_empty() {
        return Err(ApiError::bad_request("proposer_did cannot be empty"));
    }

    // Verify DAO exists and proposer is a member
    let daos = state.daos.read().await;
    let dao = daos
        .get(&dao_id)
        .ok_or_else(|| ApiError::not_found(format!("DAO {dao_id} not found")))?;
    if !dao.is_member(&req.proposer_did) {
        return Err(ApiError::unauthorized("proposer is not a DAO member"));
    }
    drop(daos);

    // Look up stored identity for signing
    let identities = state.identities.read().await;
    let identity = identities
        .get(&req.proposer_did)
        .ok_or_else(|| ApiError::not_found("identity not found — create one first"))?;

    let mut builder =
        nous_governance::proposal::ProposalBuilder::new(&dao_id, &req.title, &req.description);
    if let Some(q) = req.quorum {
        builder = builder.quorum(q);
    }
    if let Some(t) = req.threshold {
        builder = builder.threshold(t);
    }
    if let Some(days) = req.voting_days {
        builder = builder.voting_duration(chrono::Duration::days(days));
    }

    let proposal = builder.submit(identity).map_err(ApiError::from)?;
    let response = ProposalResponse::from(&proposal);

    drop(identities);

    let tally = VoteTally::new(&proposal.id, proposal.quorum, proposal.threshold);

    let pid = proposal.id.clone();
    let mut proposals = state.proposals.write().await;
    proposals.insert(pid.clone(), proposal);

    // Persist proposal to SQLite
    if let Some(p) = proposals.get(&pid) {
        state.persist_proposal(&pid, p).await;
    }
    drop(proposals);

    let tid = response.id.clone();
    let mut tallies = state.tallies.write().await;
    tallies.insert(tid.clone(), tally);

    // Persist tally to SQLite
    if let Some(t) = tallies.get(&tid) {
        state.persist_tally(&tid, t).await;
    }

    Ok(Json(response))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SimpleVoteRequest {
    pub voter_did: String,
    pub choice: String,
    pub credits: u64,
}

#[utoipa::path(
    post, path = "/api/v1/proposals/{proposal_id}/vote",
    tag = "governance",
    params(("proposal_id" = String, Path, description = "Proposal identifier")),
    request_body = SimpleVoteRequest,
    responses(
        (status = 200, description = "Vote cast"),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Proposal or identity not found")
    )
)]
pub async fn simple_vote(
    State(state): State<Arc<AppState>>,
    Path(proposal_id): Path<String>,
    Json(req): Json<SimpleVoteRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    if req.voter_did.is_empty() {
        return Err(ApiError::bad_request("voter_did cannot be empty"));
    }

    let choice = match req.choice.to_lowercase().as_str() {
        "for" => VoteChoice::For,
        "against" => VoteChoice::Against,
        "abstain" => VoteChoice::Abstain,
        _ => {
            return Err(ApiError::bad_request(
                "choice must be for, against, or abstain",
            ));
        }
    };

    // Verify proposal exists
    let proposals = state.proposals.read().await;
    if !proposals.contains_key(&proposal_id) {
        return Err(ApiError::not_found(format!(
            "proposal {proposal_id} not found"
        )));
    }
    drop(proposals);

    // Look up stored identity for signing
    let identities = state.identities.read().await;
    let identity = identities
        .get(&req.voter_did)
        .ok_or_else(|| ApiError::not_found("identity not found — create one first"))?;

    let ballot =
        Ballot::new(&proposal_id, identity, choice, req.credits).map_err(ApiError::from)?;

    drop(identities);

    let mut tallies = state.tallies.write().await;
    let tally = tallies
        .get_mut(&proposal_id)
        .ok_or_else(|| ApiError::internal("tally not initialized for proposal"))?;

    tally.cast(ballot).map_err(ApiError::from)?;

    // Persist tally to SQLite
    state.persist_tally(&proposal_id, tally).await;

    Ok(Json(MutationResponse {
        success: true,
        message: "vote cast".into(),
    }))
}

// ── Delegation Handlers ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDelegationRequest {
    pub from_did: String,
    pub to_did: String,
    pub scope_type: String,
    pub scope_id: String,
    pub expires_in_hours: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DelegationResponse {
    pub id: String,
    pub from_did: String,
    pub to_did: String,
    pub scope_type: String,
    pub scope_id: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub revoked: bool,
    pub active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DelegationListResponse {
    pub delegations: Vec<DelegationResponse>,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EffectivePowerResponse {
    pub scope_type: String,
    pub scope_id: String,
    pub power: Vec<PowerEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PowerEntry {
    pub did: String,
    pub base_credits: u64,
    pub effective_credits: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DelegationChainResponse {
    pub chain: Vec<String>,
    pub final_delegate: Option<String>,
}

fn parse_scope(scope_type: &str, scope_id: &str) -> Result<DelegationScope, ApiError> {
    match scope_type {
        "dao" => Ok(DelegationScope::Dao(scope_id.to_string())),
        "proposal" => Ok(DelegationScope::Proposal(scope_id.to_string())),
        _ => Err(ApiError::bad_request(
            "scope_type must be 'dao' or 'proposal'",
        )),
    }
}

fn delegation_to_response(d: &nous_governance::Delegation) -> DelegationResponse {
    let (scope_type, scope_id) = match &d.scope {
        DelegationScope::Dao(id) => ("dao".to_string(), id.clone()),
        DelegationScope::Proposal(id) => ("proposal".to_string(), id.clone()),
    };
    DelegationResponse {
        id: d.id.clone(),
        from_did: d.from_did.clone(),
        to_did: d.to_did.clone(),
        scope_type,
        scope_id,
        created_at: d.created_at.to_rfc3339(),
        expires_at: d.expires_at.map(|t| t.to_rfc3339()),
        revoked: d.revoked,
        active: !d.revoked && d.expires_at.is_none_or(|t| chrono::Utc::now() < t),
    }
}

#[utoipa::path(
    post, path = "/api/v1/delegations",
    tag = "governance",
    request_body = CreateDelegationRequest,
    responses(
        (status = 200, description = "Delegation created"),
        (status = 400, description = "Invalid request or cycle detected")
    )
)]
pub async fn create_delegation(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateDelegationRequest>,
) -> Result<Json<DelegationResponse>, ApiError> {
    if req.from_did.is_empty() || req.to_did.is_empty() {
        return Err(ApiError::bad_request("from_did and to_did are required"));
    }

    let scope = parse_scope(&req.scope_type, &req.scope_id)?;

    let expires_at = req
        .expires_in_hours
        .map(|h| chrono::Utc::now() + chrono::Duration::hours(h));

    let mut registry = state.delegations.write().await;
    let id = registry
        .delegate(&req.from_did, &req.to_did, scope, expires_at)
        .map_err(ApiError::from)?;

    let delegation = registry.get(&id).unwrap();
    let resp = delegation_to_response(delegation);

    // Persist delegations to SQLite
    state.persist_delegations(&registry).await;

    Ok(Json(resp))
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct DelegationQuery {
    pub scope_type: Option<String>,
    pub scope_id: Option<String>,
    pub from_did: Option<String>,
    pub to_did: Option<String>,
}

#[utoipa::path(
    get, path = "/api/v1/delegations",
    tag = "governance",
    params(DelegationQuery),
    responses((status = 200, description = "List delegations"))
)]
pub async fn list_delegations(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DelegationQuery>,
) -> Json<DelegationListResponse> {
    let registry = state.delegations.read().await;

    let delegations: Vec<DelegationResponse> =
        if let (Some(st), Some(si)) = (&query.scope_type, &query.scope_id) {
            if let Ok(scope) = parse_scope(st, si) {
                registry
                    .active_delegations(&scope)
                    .into_iter()
                    .map(delegation_to_response)
                    .collect()
            } else {
                vec![]
            }
        } else if let Some(ref from) = query.from_did {
            registry
                .delegations_from(from)
                .into_iter()
                .map(delegation_to_response)
                .collect()
        } else if let Some(ref to) = query.to_did {
            registry
                .delegations_to(to)
                .into_iter()
                .map(delegation_to_response)
                .collect()
        } else {
            vec![]
        };

    let count = delegations.len();
    Json(DelegationListResponse { delegations, count })
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RevokeDelegationRequest {
    pub requester_did: String,
}

#[utoipa::path(
    post, path = "/api/v1/delegations/{delegation_id}/revoke",
    tag = "governance",
    params(("delegation_id" = String, Path, description = "Delegation ID")),
    request_body = RevokeDelegationRequest,
    responses(
        (status = 200, description = "Delegation revoked"),
        (status = 404, description = "Delegation not found"),
        (status = 401, description = "Not authorized to revoke")
    )
)]
pub async fn revoke_delegation(
    State(state): State<Arc<AppState>>,
    Path(delegation_id): Path<String>,
    Json(req): Json<RevokeDelegationRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut registry = state.delegations.write().await;
    registry
        .revoke(&delegation_id, &req.requester_did)
        .map_err(ApiError::from)?;

    // Persist delegations to SQLite
    state.persist_delegations(&registry).await;

    Ok(Json(MutationResponse {
        success: true,
        message: format!("delegation {} revoked", delegation_id),
    }))
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct PowerQuery {
    pub scope_type: String,
    pub scope_id: String,
}

#[utoipa::path(
    get, path = "/api/v1/delegations/power",
    tag = "governance",
    params(PowerQuery),
    responses((status = 200, description = "Effective voting power for all members"))
)]
pub async fn get_effective_power(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PowerQuery>,
) -> Result<Json<EffectivePowerResponse>, ApiError> {
    let scope = parse_scope(&query.scope_type, &query.scope_id)?;

    // Get member credits from the DAO
    let dao_id = match &scope {
        DelegationScope::Dao(id) => id.clone(),
        DelegationScope::Proposal(prop_id) => {
            let proposals = state.proposals.read().await;
            proposals
                .get(prop_id)
                .map(|p| p.dao_id.clone())
                .ok_or_else(|| ApiError::not_found("proposal not found"))?
        }
    };

    let daos = state.daos.read().await;
    let dao = daos
        .get(&dao_id)
        .ok_or_else(|| ApiError::not_found("DAO not found"))?;

    let member_credits: std::collections::HashMap<String, u64> = dao
        .members
        .values()
        .map(|m| (m.did.clone(), m.credits))
        .collect();
    drop(daos);

    let registry = state.delegations.read().await;
    let effective = registry.effective_power(&member_credits, &scope);

    let power: Vec<PowerEntry> = member_credits
        .iter()
        .map(|(did, &base)| PowerEntry {
            did: did.clone(),
            base_credits: base,
            effective_credits: effective.get(did).copied().unwrap_or(0),
        })
        .collect();

    Ok(Json(EffectivePowerResponse {
        scope_type: query.scope_type,
        scope_id: query.scope_id,
        power,
    }))
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ChainQuery {
    pub did: String,
    pub scope_type: String,
    pub scope_id: String,
}

#[utoipa::path(
    get, path = "/api/v1/delegations/chain",
    tag = "governance",
    params(ChainQuery),
    responses((status = 200, description = "Delegation chain from a DID"))
)]
pub async fn get_delegation_chain(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ChainQuery>,
) -> Result<Json<DelegationChainResponse>, ApiError> {
    let scope = parse_scope(&query.scope_type, &query.scope_id)?;
    let registry = state.delegations.read().await;

    let chain = registry.delegation_chain(&query.did, &scope);
    let final_delegate = registry.resolve_delegate(&query.did, &scope);

    Ok(Json(DelegationChainResponse {
        chain,
        final_delegate,
    }))
}

// ── Execution Handlers ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct QueueExecutionRequest {
    pub proposal_id: String,
    pub actions: Vec<ActionRequest>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum ActionRequest {
    #[serde(rename = "treasury_transfer")]
    TreasuryTransfer {
        recipient_did: String,
        token: String,
        amount: u64,
    },
    #[serde(rename = "parameter_change")]
    ParameterChange { parameter: String, value: String },
    #[serde(rename = "add_member")]
    AddMember { did: String },
    #[serde(rename = "remove_member")]
    RemoveMember { did: String },
    #[serde(rename = "grant_credits")]
    GrantCredits { did: String, amount: u64 },
}

impl From<ActionRequest> for ProposalAction {
    fn from(req: ActionRequest) -> Self {
        match req {
            ActionRequest::TreasuryTransfer {
                recipient_did,
                token,
                amount,
            } => ProposalAction::TreasuryTransfer {
                recipient_did,
                token,
                amount,
            },
            ActionRequest::ParameterChange { parameter, value } => {
                ProposalAction::ParameterChange { parameter, value }
            }
            ActionRequest::AddMember { did } => ProposalAction::AddMember { did },
            ActionRequest::RemoveMember { did } => ProposalAction::RemoveMember { did },
            ActionRequest::GrantCredits { did, amount } => {
                ProposalAction::GrantCredits { did, amount }
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionResponse {
    pub id: String,
    pub proposal_id: String,
    pub dao_id: String,
    pub status: String,
    pub queued_at: String,
    pub executable_at: String,
    pub expires_at: String,
    pub executed_at: Option<String>,
    pub executor_did: Option<String>,
    pub error: Option<String>,
}

fn execution_to_response(e: &nous_governance::QueuedExecution) -> ExecutionResponse {
    ExecutionResponse {
        id: e.id.clone(),
        proposal_id: e.proposal_id.clone(),
        dao_id: e.dao_id.clone(),
        status: format!("{:?}", e.status),
        queued_at: e.queued_at.to_rfc3339(),
        executable_at: e.executable_at.to_rfc3339(),
        expires_at: e.expires_at.to_rfc3339(),
        executed_at: e.executed_at.map(|t| t.to_rfc3339()),
        executor_did: e.executor_did.clone(),
        error: e.error.clone(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionListResponse {
    pub executions: Vec<ExecutionResponse>,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionResultResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteResultResponse {
    pub execution_id: String,
    pub status: String,
    pub results: Vec<ActionResultResponse>,
}

#[utoipa::path(
    post, path = "/api/v1/executions",
    tag = "governance",
    request_body = QueueExecutionRequest,
    responses(
        (status = 200, description = "Proposal queued for execution"),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn queue_execution(
    State(state): State<Arc<AppState>>,
    Json(req): Json<QueueExecutionRequest>,
) -> Result<Json<ExecutionResponse>, ApiError> {
    if req.actions.is_empty() {
        return Err(ApiError::bad_request("at least one action is required"));
    }

    // Verify proposal exists and is passed
    let proposals = state.proposals.read().await;
    let proposal = proposals
        .get(&req.proposal_id)
        .ok_or_else(|| ApiError::not_found("proposal not found"))?;

    let dao_id = proposal.dao_id.clone();
    let status = proposal.status;
    drop(proposals);

    let actions: Vec<ProposalAction> = req.actions.into_iter().map(Into::into).collect();

    let mut engine = state.execution_engine.write().await;
    let id = engine
        .queue_proposal(&req.proposal_id, &dao_id, status, actions)
        .map_err(ApiError::from)?;

    let execution = engine.get(&id).unwrap();
    let resp = execution_to_response(execution);

    // Persist execution engine to SQLite
    state.persist_execution_engine(&engine).await;

    Ok(Json(resp))
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ExecutionQuery {
    pub dao_id: Option<String>,
    pub status: Option<String>,
}

#[utoipa::path(
    get, path = "/api/v1/executions",
    tag = "governance",
    params(ExecutionQuery),
    responses((status = 200, description = "List queued executions"))
)]
pub async fn list_executions(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ExecutionQuery>,
) -> Json<ExecutionListResponse> {
    let engine = state.execution_engine.read().await;

    let executions: Vec<ExecutionResponse> = if let Some(ref dao_id) = query.dao_id {
        engine
            .list_for_dao(dao_id)
            .into_iter()
            .map(execution_to_response)
            .collect()
    } else if query.status.as_deref() == Some("ready") {
        engine
            .ready_executions()
            .into_iter()
            .map(execution_to_response)
            .collect()
    } else {
        vec![]
    };

    let count = executions.len();
    Json(ExecutionListResponse { executions, count })
}

#[utoipa::path(
    get, path = "/api/v1/executions/{execution_id}",
    tag = "governance",
    params(("execution_id" = String, Path, description = "Execution ID")),
    responses(
        (status = 200, description = "Execution details"),
        (status = 404, description = "Execution not found")
    )
)]
pub async fn get_execution(
    State(state): State<Arc<AppState>>,
    Path(execution_id): Path<String>,
) -> Result<Json<ExecutionResponse>, ApiError> {
    let engine = state.execution_engine.read().await;
    let execution = engine
        .get(&execution_id)
        .ok_or_else(|| ApiError::not_found("execution not found"))?;
    Ok(Json(execution_to_response(execution)))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ExecuteRequest {
    pub executor_did: String,
}

#[utoipa::path(
    post, path = "/api/v1/executions/{execution_id}/execute",
    tag = "governance",
    params(("execution_id" = String, Path, description = "Execution ID")),
    request_body = ExecuteRequest,
    responses(
        (status = 200, description = "Execution completed"),
        (status = 400, description = "Timelock not expired or already executed"),
        (status = 404, description = "Execution not found")
    )
)]
pub async fn execute(
    State(state): State<Arc<AppState>>,
    Path(execution_id): Path<String>,
    Json(req): Json<ExecuteRequest>,
) -> Result<Json<ExecuteResultResponse>, ApiError> {
    let mut engine = state.execution_engine.write().await;
    let results = engine
        .execute(&execution_id, &req.executor_did)
        .map_err(ApiError::from)?;

    let execution = engine.get(&execution_id).unwrap();
    let status = format!("{:?}", execution.status);

    // Persist execution engine to SQLite
    state.persist_execution_engine(&engine).await;

    let action_results: Vec<ActionResultResponse> = results
        .into_iter()
        .map(|r| ActionResultResponse {
            success: r.success,
            message: r.message,
        })
        .collect();

    Ok(Json(ExecuteResultResponse {
        execution_id,
        status,
        results: action_results,
    }))
}

#[utoipa::path(
    post, path = "/api/v1/executions/{execution_id}/cancel",
    tag = "governance",
    params(("execution_id" = String, Path, description = "Execution ID")),
    responses(
        (status = 200, description = "Execution cancelled"),
        (status = 400, description = "Cannot cancel"),
        (status = 404, description = "Execution not found")
    )
)]
pub async fn cancel_execution(
    State(state): State<Arc<AppState>>,
    Path(execution_id): Path<String>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut engine = state.execution_engine.write().await;
    engine.cancel(&execution_id).map_err(ApiError::from)?;

    // Persist execution engine to SQLite
    state.persist_execution_engine(&engine).await;

    Ok(Json(MutationResponse {
        success: true,
        message: format!("execution {} cancelled", execution_id),
    }))
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiConfig;
    use crate::router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use nous_governance::VoteChoice;
    use tower::ServiceExt;

    async fn test_app() -> axum::Router {
        router(ApiConfig::default())
    }

    fn json_request(method: &str, uri: &str, body: serde_json::Value) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap()
    }

    async fn create_test_dao(app: &axum::Router) -> DaoResponse {
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/daos",
                serde_json::json!({
                    "founder_did": "did:key:zFounder",
                    "name": "TestDAO",
                    "description": "A test DAO"
                }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn create_and_get_dao() {
        let app = test_app().await;
        let dao = create_test_dao(&app).await;

        assert_eq!(dao.name, "TestDAO");
        assert_eq!(dao.member_count, 1);
        assert!(dao.id.starts_with("dao:"));

        // Get DAO
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/daos/{}", dao.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let detail: DaoDetailResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(detail.name, "TestDAO");
        assert_eq!(detail.members.len(), 1);
    }

    #[tokio::test]
    async fn list_daos_empty() {
        let app = test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/daos")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let list: DaoListResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(list.count, 0);
    }

    #[tokio::test]
    async fn create_dao_rejects_empty_name() {
        let app = test_app().await;

        let response = app
            .oneshot(json_request(
                "POST",
                "/api/v1/daos",
                serde_json::json!({
                    "founder_did": "did:key:z123",
                    "name": "",
                    "description": "Bad"
                }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn add_and_remove_member() {
        let app = test_app().await;
        let dao = create_test_dao(&app).await;

        // Add member
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/daos/{}/members", dao.id),
                serde_json::json!({ "did": "did:key:zMember1" }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify member count
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/daos/{}", dao.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let detail: DaoDetailResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(detail.members.len(), 2);

        // Remove member
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/daos/{}/members/did:key:zMember1", dao.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn dao_not_found() {
        let app = test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/daos/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn submit_and_get_proposal() {
        let app = test_app().await;
        let dao = create_test_dao(&app).await;

        // Create a signed proposal
        let proposer = nous_identity::Identity::generate();
        let proposal = nous_governance::proposal::ProposalBuilder::new(
            &dao.id,
            "Fund research",
            "Allocate 500 tokens to research",
        )
        .submit(&proposer)
        .unwrap();

        // We need the proposer to be a DAO member — update the DAO
        // Add proposer as member first
        let _ = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/daos/{}/members", dao.id),
                serde_json::json!({ "did": proposer.did() }),
            ))
            .await
            .unwrap();

        let proposal_json = serde_json::to_value(&proposal).unwrap();

        let response = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/proposals", proposal_json))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let prop_resp: ProposalResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(prop_resp.title, "Fund research");

        // Get proposal
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/proposals/{}", prop_resp.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn list_proposals_by_dao() {
        let app = test_app().await;
        let dao = create_test_dao(&app).await;

        let proposer = nous_identity::Identity::generate();
        let _ = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/daos/{}/members", dao.id),
                serde_json::json!({ "did": proposer.did() }),
            ))
            .await
            .unwrap();

        let proposal = nous_governance::proposal::ProposalBuilder::new(
            &dao.id,
            "Test proposal",
            "Description",
        )
        .submit(&proposer)
        .unwrap();

        let _ = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/proposals",
                serde_json::to_value(&proposal).unwrap(),
            ))
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/proposals?dao_id={}", dao.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let list: ProposalListResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(list.count, 1);
    }

    #[tokio::test]
    async fn cast_vote_and_tally() {
        let app = test_app().await;
        let dao = create_test_dao(&app).await;

        // Add proposer and submit proposal
        let proposer = nous_identity::Identity::generate();
        let _ = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/daos/{}/members", dao.id),
                serde_json::json!({ "did": proposer.did() }),
            ))
            .await
            .unwrap();

        let proposal =
            nous_governance::proposal::ProposalBuilder::new(&dao.id, "Vote test", "Test voting")
                .submit(&proposer)
                .unwrap();

        let proposal_id = proposal.id.clone();

        let _ = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/proposals",
                serde_json::to_value(&proposal).unwrap(),
            ))
            .await
            .unwrap();

        // Cast a vote
        let voter = nous_identity::Identity::generate();
        let ballot = Ballot::new(&proposal_id, &voter, VoteChoice::For, 9).unwrap();

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/votes",
                serde_json::to_value(&ballot).unwrap(),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Get tally
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/votes/{}", proposal_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let result: VoteResultResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(result.votes_for, 3); // sqrt(9) = 3
        assert_eq!(result.total_voters, 1);
    }

    #[tokio::test]
    async fn private_vote_and_tally() {
        let app = test_app().await;
        let dao = create_test_dao(&app).await;

        let proposer = nous_identity::Identity::generate();
        let _ = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/daos/{}/members", dao.id),
                serde_json::json!({ "did": proposer.did() }),
            ))
            .await
            .unwrap();

        let proposal = nous_governance::proposal::ProposalBuilder::new(
            &dao.id,
            "ZK vote test",
            "Test private voting",
        )
        .submit(&proposer)
        .unwrap();

        let proposal_id = proposal.id.clone();

        let _ = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/proposals",
                serde_json::to_value(&proposal).unwrap(),
            ))
            .await
            .unwrap();

        // Submit a private vote
        let (committed_vote, _opening) =
            nous_governance::commit_vote(&proposal_id, "did:key:zVoter", VoteChoice::For, 10)
                .unwrap();

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/votes/private",
                serde_json::to_value(&committed_vote).unwrap(),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Get private tally
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/votes/private/{}", proposal_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let result: PrivateTallyResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(result.total_votes, 1);
        assert_eq!(result.votes_for, 1);
        assert!(result.all_proofs_valid);
    }

    #[tokio::test]
    async fn vote_nonexistent_proposal() {
        let app = test_app().await;

        let voter = nous_identity::Identity::generate();
        let ballot = Ballot::new("prop:nonexistent", &voter, VoteChoice::For, 1).unwrap();

        let response = app
            .oneshot(json_request(
                "POST",
                "/api/v1/votes",
                serde_json::to_value(&ballot).unwrap(),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn proposal_not_found() {
        let app = test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/proposals/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn private_tally_empty() {
        let app = test_app().await;
        let dao = create_test_dao(&app).await;

        let proposer = nous_identity::Identity::generate();
        let _ = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/daos/{}/members", dao.id),
                serde_json::json!({ "did": proposer.did() }),
            ))
            .await
            .unwrap();

        let proposal =
            nous_governance::proposal::ProposalBuilder::new(&dao.id, "Empty tally", "No votes yet")
                .submit(&proposer)
                .unwrap();

        let proposal_id = proposal.id.clone();

        let _ = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/proposals",
                serde_json::to_value(&proposal).unwrap(),
            ))
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/votes/private/{}", proposal_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let result: PrivateTallyResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(result.total_votes, 0);
    }

    // Helper: create identity via API, returns DID
    async fn create_test_identity(app: &axum::Router) -> String {
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/identities",
                serde_json::json!({ "display_name": "Test User" }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        body["did"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn convenience_create_proposal() {
        let app = test_app().await;
        let did = create_test_identity(&app).await;

        // Create DAO with this identity as founder
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/daos",
                serde_json::json!({
                    "founder_did": &did,
                    "name": "ConvDAO",
                    "description": "Test convenience endpoints"
                }),
            ))
            .await
            .unwrap();

        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let dao: DaoResponse = serde_json::from_slice(&bytes).unwrap();

        // Create proposal via convenience endpoint
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/daos/{}/proposals", dao.id),
                serde_json::json!({
                    "proposer_did": &did,
                    "title": "Fund development",
                    "description": "Allocate 1000 tokens to dev team"
                }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let prop: ProposalResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(prop.title, "Fund development");
        assert_eq!(prop.dao_id, dao.id);
    }

    #[tokio::test]
    async fn convenience_vote_on_proposal() {
        let app = test_app().await;
        let proposer_did = create_test_identity(&app).await;
        let voter_did = create_test_identity(&app).await;

        // Create DAO
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/daos",
                serde_json::json!({
                    "founder_did": &proposer_did,
                    "name": "VoteDAO",
                    "description": "Test voting"
                }),
            ))
            .await
            .unwrap();

        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let dao: DaoResponse = serde_json::from_slice(&bytes).unwrap();

        // Add voter as member
        let _ = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/daos/{}/members", dao.id),
                serde_json::json!({ "did": &voter_did }),
            ))
            .await
            .unwrap();

        // Create proposal via convenience endpoint
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/daos/{}/proposals", dao.id),
                serde_json::json!({
                    "proposer_did": &proposer_did,
                    "title": "Vote test",
                    "description": "Testing simple vote"
                }),
            ))
            .await
            .unwrap();

        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let prop: ProposalResponse = serde_json::from_slice(&bytes).unwrap();

        // Cast vote via convenience endpoint
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/proposals/{}/vote", prop.id),
                serde_json::json!({
                    "voter_did": &voter_did,
                    "choice": "for",
                    "credits": 16
                }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Check tally
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/votes/{}", prop.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let tally: VoteResultResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(tally.votes_for, 4); // sqrt(16) = 4
        assert_eq!(tally.total_voters, 1);
    }

    #[tokio::test]
    async fn convenience_proposal_rejects_non_member() {
        let app = test_app().await;
        let founder_did = create_test_identity(&app).await;
        let outsider_did = create_test_identity(&app).await;

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/daos",
                serde_json::json!({
                    "founder_did": &founder_did,
                    "name": "ClosedDAO",
                    "description": "Members only"
                }),
            ))
            .await
            .unwrap();

        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let dao: DaoResponse = serde_json::from_slice(&bytes).unwrap();

        // Try to create proposal as non-member
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/daos/{}/proposals", dao.id),
                serde_json::json!({
                    "proposer_did": &outsider_did,
                    "title": "Rejected",
                    "description": "Should fail"
                }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn convenience_vote_invalid_choice() {
        let app = test_app().await;
        let did = create_test_identity(&app).await;

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/proposals/prop:fake/vote",
                serde_json::json!({
                    "voter_did": &did,
                    "choice": "maybe",
                    "credits": 1
                }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn convenience_proposal_with_custom_params() {
        let app = test_app().await;
        let did = create_test_identity(&app).await;

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/daos",
                serde_json::json!({
                    "founder_did": &did,
                    "name": "CustomDAO",
                    "description": "Test custom params"
                }),
            ))
            .await
            .unwrap();

        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let dao: DaoResponse = serde_json::from_slice(&bytes).unwrap();

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/daos/{}/proposals", dao.id),
                serde_json::json!({
                    "proposer_did": &did,
                    "title": "Custom vote",
                    "description": "Custom parameters",
                    "quorum": 0.33,
                    "threshold": 0.67,
                    "voting_days": 3
                }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let prop: ProposalResponse = serde_json::from_slice(&bytes).unwrap();
        assert!((prop.quorum - 0.33).abs() < f64::EPSILON);
        assert!((prop.threshold - 0.67).abs() < f64::EPSILON);
    }

    // ── Delegation Tests ──────────────────────────────────────────────────

    #[tokio::test]
    async fn create_and_list_delegation() {
        let app = test_app().await;

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/delegations",
                serde_json::json!({
                    "from_did": "did:key:alice",
                    "to_did": "did:key:bob",
                    "scope_type": "dao",
                    "scope_id": "dao:test"
                }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let delegation: DelegationResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(delegation.from_did, "did:key:alice");
        assert_eq!(delegation.to_did, "did:key:bob");
        assert!(delegation.active);

        // List delegations by scope
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/delegations?scope_type=dao&scope_id=dao:test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let list: DelegationListResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(list.count, 1);
    }

    #[tokio::test]
    async fn delegation_self_rejected() {
        let app = test_app().await;

        let response = app
            .oneshot(json_request(
                "POST",
                "/api/v1/delegations",
                serde_json::json!({
                    "from_did": "did:key:alice",
                    "to_did": "did:key:alice",
                    "scope_type": "dao",
                    "scope_id": "dao:test"
                }),
            ))
            .await
            .unwrap();

        assert_ne!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn revoke_delegation_api() {
        let app = test_app().await;

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/delegations",
                serde_json::json!({
                    "from_did": "did:key:alice",
                    "to_did": "did:key:bob",
                    "scope_type": "dao",
                    "scope_id": "dao:test"
                }),
            ))
            .await
            .unwrap();

        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let delegation: DelegationResponse = serde_json::from_slice(&bytes).unwrap();

        let response = app
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/delegations/{}/revoke", delegation.id),
                serde_json::json!({ "requester_did": "did:key:alice" }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn effective_power_endpoint() {
        let app = test_app().await;

        // Create a DAO first
        let dao = create_test_dao(&app).await;

        // Add a member
        let _ = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/daos/{}/members", dao.id),
                serde_json::json!({ "did": "did:key:zMember" }),
            ))
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/v1/delegations/power?scope_type=dao&scope_id={}",
                        dao.id
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let power: EffectivePowerResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(power.power.len(), 2);
    }

    #[tokio::test]
    async fn delegation_chain_endpoint() {
        let app = test_app().await;

        // Create delegation chain: alice → bob
        let _ = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/delegations",
                serde_json::json!({
                    "from_did": "did:key:alice",
                    "to_did": "did:key:bob",
                    "scope_type": "dao",
                    "scope_id": "dao:test"
                }),
            ))
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/delegations/chain?did=did:key:alice&scope_type=dao&scope_id=dao:test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let chain: DelegationChainResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(chain.chain, vec!["did:key:alice", "did:key:bob"]);
        assert_eq!(chain.final_delegate, Some("did:key:bob".to_string()));
    }

    // ── Execution Tests ──────────────────────────────────────────────────

    #[tokio::test]
    async fn queue_and_get_execution() {
        let app = test_app().await;
        let dao = create_test_dao(&app).await;

        // Create identity and proposal
        let _ = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/identities",
                serde_json::json!({ "display_name": "Founder" }),
            ))
            .await
            .unwrap();

        // We need to create a proposal that is Passed status.
        // Since the convenience endpoint creates Active proposals, we'll test the queue endpoint
        // which should reject non-Passed proposals.
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/executions",
                serde_json::json!({
                    "proposal_id": "prop:nonexistent",
                    "actions": [{ "type": "add_member", "did": "did:key:znew" }]
                }),
            ))
            .await
            .unwrap();

        // Should fail because proposal doesn't exist
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn execution_rejects_empty_actions() {
        let app = test_app().await;

        let response = app
            .oneshot(json_request(
                "POST",
                "/api/v1/executions",
                serde_json::json!({
                    "proposal_id": "prop:test",
                    "actions": []
                }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn cancel_execution_api() {
        let app = test_app().await;

        // Cancel a nonexistent execution — should 404
        let response = app
            .oneshot(json_request(
                "POST",
                "/api/v1/executions/exec:nonexistent/cancel",
                serde_json::json!({}),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
