use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use nous_identity::reputation::ReputationCategory;
use nous_identity::{Credential, CredentialBuilder, Identity, Reputation};

use crate::error::ApiError;
use crate::state::AppState;

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateIdentityRequest {
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IdentityResponse {
    pub did: String,
    pub display_name: Option<String>,
    pub signing_key_type: String,
    pub exchange_key_type: String,
}

impl From<&Identity> for IdentityResponse {
    fn from(id: &Identity) -> Self {
        Self {
            did: id.did().to_string(),
            display_name: id.display_name().map(String::from),
            signing_key_type: "ed25519".to_string(),
            exchange_key_type: "x25519".to_string(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DocumentResponse {
    pub did: String,
    pub document: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct IssueCredentialRequest {
    pub subject_did: String,
    pub issuer_did: String,
    pub credential_type: String,
    pub claims: serde_json::Value,
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CredentialResponse {
    pub id: String,
    pub credential_type: Vec<String>,
    pub issuer: String,
    pub subject: String,
    pub issuance_date: String,
    pub expiration_date: Option<String>,
    pub expired: bool,
    pub claims: serde_json::Value,
}

impl From<&Credential> for CredentialResponse {
    fn from(c: &Credential) -> Self {
        Self {
            id: c.id.clone(),
            credential_type: c.r#type.clone(),
            issuer: c.issuer.clone(),
            subject: c.credential_subject.id.clone(),
            issuance_date: c.issuance_date.to_rfc3339(),
            expiration_date: c.expiration_date.map(|d| d.to_rfc3339()),
            expired: c.is_expired(),
            claims: c.credential_subject.claims.clone(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ReputationEventRequest {
    pub issuer_did: String,
    pub category: String,
    pub delta: i32,
    pub reason: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReputationResponse {
    pub did: String,
    pub total_score: i64,
    pub scores: std::collections::HashMap<String, i64>,
    pub event_count: usize,
}

// ── Handlers ────────────────────────────────────────────────────────

#[utoipa::path(
    post, path = "/api/v1/identities",
    tag = "identity",
    request_body = CreateIdentityRequest,
    responses(
        (status = 200, description = "Identity created", body = IdentityResponse)
    )
)]
pub async fn create_identity(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateIdentityRequest>,
) -> Result<Json<IdentityResponse>, ApiError> {
    let mut identity = Identity::generate();
    if let Some(ref name) = req.display_name {
        identity = identity.with_display_name(name);
    }

    let resp = IdentityResponse::from(&identity);
    let did = identity.did().to_string();

    let mut identities = state.identities.write().await;
    identities.insert(did.clone(), identity);

    // Initialize reputation tracker
    let mut reputations = state.reputations.write().await;
    reputations.insert(did, Reputation::new(&resp.did));

    Ok(Json(resp))
}

#[utoipa::path(
    get, path = "/api/v1/identities/{did}",
    tag = "identity",
    params(("did" = String, Path, description = "DID")),
    responses(
        (status = 200, description = "Identity details", body = IdentityResponse),
        (status = 404, description = "Identity not found")
    )
)]
pub async fn get_identity(
    State(state): State<Arc<AppState>>,
    Path(did): Path<String>,
) -> Result<Json<IdentityResponse>, ApiError> {
    let identities = state.identities.read().await;
    identities
        .get(&did)
        .map(|id| Json(IdentityResponse::from(id)))
        .ok_or_else(|| ApiError::not_found(format!("identity {did} not found")))
}

#[utoipa::path(
    get, path = "/api/v1/identities/{did}/document",
    tag = "identity",
    params(("did" = String, Path, description = "DID")),
    responses(
        (status = 200, description = "DID document", body = DocumentResponse),
        (status = 404, description = "Identity not found")
    )
)]
pub async fn get_document(
    State(state): State<Arc<AppState>>,
    Path(did): Path<String>,
) -> Result<Json<DocumentResponse>, ApiError> {
    let identities = state.identities.read().await;
    let identity = identities
        .get(&did)
        .ok_or_else(|| ApiError::not_found(format!("identity {did} not found")))?;

    let doc = identity.document();
    let doc_json = serde_json::to_value(doc).map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(DocumentResponse {
        did: did.clone(),
        document: doc_json,
    }))
}

#[utoipa::path(
    post, path = "/api/v1/identities/{did}/credentials",
    tag = "identity",
    params(("did" = String, Path, description = "Subject DID")),
    request_body = IssueCredentialRequest,
    responses(
        (status = 200, description = "Credential issued", body = CredentialResponse),
        (status = 404, description = "Issuer identity not found")
    )
)]
pub async fn issue_credential(
    State(state): State<Arc<AppState>>,
    Path(_did): Path<String>,
    Json(req): Json<IssueCredentialRequest>,
) -> Result<Json<CredentialResponse>, ApiError> {
    let identities = state.identities.read().await;
    let issuer = identities
        .get(&req.issuer_did)
        .ok_or_else(|| ApiError::not_found(format!("issuer {} not found", req.issuer_did)))?;

    let mut builder = CredentialBuilder::new(&req.subject_did)
        .add_type(&req.credential_type)
        .claims(req.claims);

    if let Some(ref expires) = req.expires_at {
        let dt = expires
            .parse::<chrono::DateTime<chrono::Utc>>()
            .map_err(|_| ApiError::bad_request("invalid expires_at format"))?;
        builder = builder.expires_at(dt);
    }

    let credential = builder
        .issue(issuer)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let resp = CredentialResponse::from(&credential);

    let mut credentials = state.credentials.write().await;
    credentials
        .entry(req.subject_did.clone())
        .or_default()
        .push(credential);

    Ok(Json(resp))
}

#[utoipa::path(
    get, path = "/api/v1/identities/{did}/credentials",
    tag = "identity",
    params(("did" = String, Path, description = "Subject DID")),
    responses((status = 200, description = "Credentials for identity"))
)]
pub async fn list_credentials(
    State(state): State<Arc<AppState>>,
    Path(did): Path<String>,
) -> Result<Json<Vec<CredentialResponse>>, ApiError> {
    let credentials = state.credentials.read().await;
    let result: Vec<CredentialResponse> = credentials
        .get(&did)
        .map(|creds| creds.iter().map(CredentialResponse::from).collect())
        .unwrap_or_default();
    Ok(Json(result))
}

#[utoipa::path(
    post, path = "/api/v1/credentials/{credential_id}/verify",
    tag = "identity",
    params(("credential_id" = String, Path, description = "Credential ID")),
    responses(
        (status = 200, description = "Verification result"),
        (status = 404, description = "Credential not found")
    )
)]
pub async fn verify_credential(
    State(state): State<Arc<AppState>>,
    Path(credential_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let credentials = state.credentials.read().await;
    for creds in credentials.values() {
        if let Some(cred) = creds.iter().find(|c| c.id == credential_id) {
            let valid = cred.verify().is_ok();
            let expired = cred.is_expired();
            return Ok(Json(serde_json::json!({
                "credential_id": credential_id,
                "valid": valid && !expired,
                "signature_valid": valid,
                "expired": expired,
            })));
        }
    }
    Err(ApiError::not_found(format!(
        "credential {credential_id} not found"
    )))
}

fn parse_category(s: &str) -> Result<ReputationCategory, ApiError> {
    match s.to_lowercase().as_str() {
        "governance" => Ok(ReputationCategory::Governance),
        "messaging" => Ok(ReputationCategory::Messaging),
        "trading" => Ok(ReputationCategory::Trading),
        "moderation" => Ok(ReputationCategory::Moderation),
        "development" => Ok(ReputationCategory::Development),
        "general" => Ok(ReputationCategory::General),
        _ => Err(ApiError::bad_request(format!("unknown category: {s}"))),
    }
}

fn category_name(cat: &ReputationCategory) -> &'static str {
    match cat {
        ReputationCategory::Governance => "governance",
        ReputationCategory::Messaging => "messaging",
        ReputationCategory::Trading => "trading",
        ReputationCategory::Moderation => "moderation",
        ReputationCategory::Development => "development",
        ReputationCategory::General => "general",
    }
}

#[utoipa::path(
    get, path = "/api/v1/identities/{did}/reputation",
    tag = "identity",
    params(("did" = String, Path, description = "DID")),
    responses(
        (status = 200, description = "Reputation scores", body = ReputationResponse),
        (status = 404, description = "Identity not found")
    )
)]
pub async fn get_reputation(
    State(state): State<Arc<AppState>>,
    Path(did): Path<String>,
) -> Result<Json<ReputationResponse>, ApiError> {
    let reputations = state.reputations.read().await;
    let reputation = reputations
        .get(&did)
        .ok_or_else(|| ApiError::not_found(format!("reputation for {did} not found")))?;

    let categories = [
        ReputationCategory::Governance,
        ReputationCategory::Messaging,
        ReputationCategory::Trading,
        ReputationCategory::Moderation,
        ReputationCategory::Development,
        ReputationCategory::General,
    ];

    let scores: std::collections::HashMap<String, i64> = categories
        .iter()
        .map(|cat| (category_name(cat).to_string(), reputation.score(*cat)))
        .collect();

    Ok(Json(ReputationResponse {
        did,
        total_score: reputation.total_score(),
        scores,
        event_count: reputation.events().len(),
    }))
}

#[utoipa::path(
    post, path = "/api/v1/identities/{did}/reputation",
    tag = "identity",
    params(("did" = String, Path, description = "Subject DID")),
    request_body = ReputationEventRequest,
    responses(
        (status = 200, description = "Reputation event applied"),
        (status = 404, description = "Identity not found")
    )
)]
pub async fn add_reputation_event(
    State(state): State<Arc<AppState>>,
    Path(did): Path<String>,
    Json(req): Json<ReputationEventRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let category = parse_category(&req.category)?;

    let identities = state.identities.read().await;
    let issuer = identities
        .get(&req.issuer_did)
        .ok_or_else(|| ApiError::not_found(format!("issuer {} not found", req.issuer_did)))?;

    let event = Reputation::issue_event(issuer, &did, category, req.delta, &req.reason)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    drop(identities);

    let mut reputations = state.reputations.write().await;
    let reputation = reputations
        .entry(did.clone())
        .or_insert_with(|| Reputation::new(&did));

    reputation
        .apply(&event)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "subject": did,
        "category": req.category,
        "delta": req.delta,
        "new_total": reputation.total_score(),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiConfig;
    use crate::router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn test_app() -> axum::Router {
        router(ApiConfig::default())
    }

    fn json_request(method: &str, uri: &str, body: &serde_json::Value) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(body).unwrap()))
            .unwrap()
    }

    async fn parse_json(response: axum::http::Response<Body>) -> serde_json::Value {
        let body = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn create_and_get_identity() {
        let app = test_app().await;
        let req = serde_json::json!({"display_name": "alice"});

        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/identities", &req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let json = parse_json(resp).await;
        assert_eq!(json["display_name"], "alice");
        assert_eq!(json["signing_key_type"], "ed25519");

        let did = json["did"].as_str().unwrap();
        assert!(did.starts_with("did:key:z"));

        // Fetch it back
        let uri = format!("/api/v1/identities/{did}");
        let resp = app
            .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let json = parse_json(resp).await;
        assert_eq!(json["display_name"], "alice");
    }

    #[tokio::test]
    async fn get_nonexistent_identity() {
        let app = test_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/identities/did:key:nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_did_document() {
        let app = test_app().await;
        let req = serde_json::json!({"display_name": "bob"});

        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/identities", &req))
            .await
            .unwrap();
        let json = parse_json(resp).await;
        let did = json["did"].as_str().unwrap();

        let uri = format!("/api/v1/identities/{did}/document");
        let resp = app
            .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let json = parse_json(resp).await;
        assert!(
            json["document"]["id"]
                .as_str()
                .unwrap()
                .starts_with("did:key:z")
        );
    }

    #[tokio::test]
    async fn issue_and_list_credentials() {
        let app = test_app().await;

        // Create issuer identity
        let req = serde_json::json!({"display_name": "issuer"});
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/identities", &req))
            .await
            .unwrap();
        let issuer = parse_json(resp).await;
        let issuer_did = issuer["did"].as_str().unwrap();

        // Create subject identity
        let req = serde_json::json!({"display_name": "subject"});
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/identities", &req))
            .await
            .unwrap();
        let subject = parse_json(resp).await;
        let subject_did = subject["did"].as_str().unwrap();

        // Issue credential
        let cred_req = serde_json::json!({
            "subject_did": subject_did,
            "issuer_did": issuer_did,
            "credential_type": "AgeVerification",
            "claims": {"age_over": 18}
        });
        let uri = format!("/api/v1/identities/{subject_did}/credentials");
        let resp = app
            .clone()
            .oneshot(json_request("POST", &uri, &cred_req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let json = parse_json(resp).await;
        assert_eq!(json["subject"], subject_did);
        assert!(!json["expired"].as_bool().unwrap());

        // List credentials
        let resp = app
            .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let creds: Vec<serde_json::Value> =
            serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
        assert_eq!(creds.len(), 1);
    }

    #[tokio::test]
    async fn verify_credential_endpoint() {
        let app = test_app().await;

        // Create issuer
        let resp = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/identities",
                &serde_json::json!({}),
            ))
            .await
            .unwrap();
        let issuer = parse_json(resp).await;
        let issuer_did = issuer["did"].as_str().unwrap();

        // Create subject
        let resp = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/identities",
                &serde_json::json!({}),
            ))
            .await
            .unwrap();
        let subject = parse_json(resp).await;
        let subject_did = subject["did"].as_str().unwrap();

        // Issue
        let cred_req = serde_json::json!({
            "subject_did": subject_did,
            "issuer_did": issuer_did,
            "credential_type": "Membership",
            "claims": {"org": "nous"}
        });
        let uri = format!("/api/v1/identities/{subject_did}/credentials");
        let resp = app
            .clone()
            .oneshot(json_request("POST", &uri, &cred_req))
            .await
            .unwrap();
        let cred = parse_json(resp).await;
        let cred_id = cred["id"].as_str().unwrap();

        // Verify
        let verify_uri = format!("/api/v1/credentials/{cred_id}/verify");
        let resp = app
            .oneshot(json_request("POST", &verify_uri, &serde_json::json!({})))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let json = parse_json(resp).await;
        assert!(json["valid"].as_bool().unwrap());
        assert!(json["signature_valid"].as_bool().unwrap());
        assert!(!json["expired"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn reputation_lifecycle() {
        let app = test_app().await;

        // Create issuer identity
        let resp = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/identities",
                &serde_json::json!({"display_name": "voter"}),
            ))
            .await
            .unwrap();
        let issuer = parse_json(resp).await;
        let issuer_did = issuer["did"].as_str().unwrap();

        // Create subject identity
        let resp = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/identities",
                &serde_json::json!({"display_name": "subject"}),
            ))
            .await
            .unwrap();
        let subject = parse_json(resp).await;
        let subject_did = subject["did"].as_str().unwrap();

        // Get initial reputation
        let uri = format!("/api/v1/identities/{subject_did}/reputation");
        let resp = app
            .clone()
            .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let json = parse_json(resp).await;
        assert_eq!(json["total_score"], 0);

        // Add reputation event
        let event_req = serde_json::json!({
            "issuer_did": issuer_did,
            "category": "governance",
            "delta": 10,
            "reason": "excellent proposal"
        });
        let resp = app
            .clone()
            .oneshot(json_request("POST", &uri, &event_req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let json = parse_json(resp).await;
        assert_eq!(json["new_total"], 10);

        // Verify reputation updated
        let resp = app
            .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let json = parse_json(resp).await;
        assert_eq!(json["total_score"], 10);
        assert_eq!(json["scores"]["governance"], 10);
        assert_eq!(json["event_count"], 1);
    }

    #[tokio::test]
    async fn invalid_reputation_category() {
        let app = test_app().await;

        let resp = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/identities",
                &serde_json::json!({}),
            ))
            .await
            .unwrap();
        let id = parse_json(resp).await;
        let did = id["did"].as_str().unwrap();

        let event_req = serde_json::json!({
            "issuer_did": did,
            "category": "invalid",
            "delta": 5,
            "reason": "test"
        });
        let uri = format!("/api/v1/identities/{did}/reputation");
        let resp = app
            .oneshot(json_request("POST", &uri, &event_req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn empty_credentials_list() {
        let app = test_app().await;

        let resp = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/identities",
                &serde_json::json!({}),
            ))
            .await
            .unwrap();
        let id = parse_json(resp).await;
        let did = id["did"].as_str().unwrap();

        let uri = format!("/api/v1/identities/{did}/credentials");
        let resp = app
            .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let creds: Vec<serde_json::Value> =
            serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
        assert!(creds.is_empty());
    }
}
