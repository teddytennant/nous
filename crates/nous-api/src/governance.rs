use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use nous_governance::{
    Ballot, CommittedVote, Dao, Proposal, VoteChoice, VoteResult, VoteTally,
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

    let mut daos = state.daos.write().await;
    daos.insert(dao.id.clone(), dao);

    Ok(Json(response))
}

#[utoipa::path(
    get, path = "/api/v1/daos",
    tag = "governance",
    responses((status = 200, description = "List of all DAOs"))
)]
pub async fn list_daos(
    State(state): State<Arc<AppState>>,
) -> Json<DaoListResponse> {
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

    let mut proposals = state.proposals.write().await;
    proposals.insert(proposal.id.clone(), proposal);
    drop(proposals);

    let mut tallies = state.tallies.write().await;
    tallies.insert(response.id.clone(), tally);

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
            if let Some(ref dao_id) = query.dao_id {
                if &p.dao_id != dao_id {
                    return false;
                }
            }
            if let Some(ref status) = query.status {
                let status_str = format!("{:?}", p.status);
                if &status_str != status {
                    return false;
                }
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

    tally.cast(ballot).map_err(ApiError::from)?;

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
    let eligible_voters = daos
        .get(&dao_id)
        .map(|d| d.member_count())
        .unwrap_or(0);
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

    let mut private_votes = state.private_votes.write().await;
    let votes = private_votes
        .entry(vote.proposal_id.clone())
        .or_default();
    votes.push(vote);

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

    let mut builder = nous_governance::proposal::ProposalBuilder::new(
        &dao_id,
        &req.title,
        &req.description,
    );
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

    let mut proposals = state.proposals.write().await;
    proposals.insert(proposal.id.clone(), proposal);
    drop(proposals);

    let mut tallies = state.tallies.write().await;
    tallies.insert(response.id.clone(), tally);

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
        _ => return Err(ApiError::bad_request("choice must be for, against, or abstain")),
    };

    // Verify proposal exists
    let proposals = state.proposals.read().await;
    if !proposals.contains_key(&proposal_id) {
        return Err(ApiError::not_found(format!("proposal {proposal_id} not found")));
    }
    drop(proposals);

    // Look up stored identity for signing
    let identities = state.identities.read().await;
    let identity = identities
        .get(&req.voter_did)
        .ok_or_else(|| ApiError::not_found("identity not found — create one first"))?;

    let ballot = Ballot::new(&proposal_id, identity, choice, req.credits)
        .map_err(ApiError::from)?;

    drop(identities);

    let mut tallies = state.tallies.write().await;
    let tally = tallies
        .get_mut(&proposal_id)
        .ok_or_else(|| ApiError::internal("tally not initialized for proposal"))?;

    tally.cast(ballot).map_err(ApiError::from)?;

    Ok(Json(MutationResponse {
        success: true,
        message: "vote cast".into(),
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

        let proposal = nous_governance::proposal::ProposalBuilder::new(
            &dao.id,
            "Vote test",
            "Test voting",
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

        let proposal = nous_governance::proposal::ProposalBuilder::new(
            &dao.id,
            "Empty tally",
            "No votes yet",
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
}
