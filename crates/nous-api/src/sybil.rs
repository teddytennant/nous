//! Sybil resistance scoring API endpoints.
//!
//! Evaluates identity trustworthiness for governance eligibility.
//! Scores are computed on-demand from provided evidence — no server
//! state is required.

use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

use nous_governance::sybil::{self, SybilScorer, TrustFactor};

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct ScoreRequest {
    /// The DID to score.
    pub did: String,
    /// Trust evidence: map of factor name to raw numeric value.
    /// Valid factors: "identity_age", "social_vouches", "on_chain_activity",
    /// "stake_amount", "device_binding", "credential_count".
    pub evidence: HashMap<String, f64>,
    /// Optional custom eligibility threshold (0.0–1.0). Defaults to 0.3.
    pub threshold: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ScoreResponse {
    pub did: String,
    /// Composite score from 0.0 (untrusted) to 1.0 (fully trusted).
    pub score: f64,
    /// Whether the identity meets the eligibility threshold.
    pub eligible: bool,
    /// The threshold used.
    pub threshold: f64,
    /// Per-factor scores (0.0–1.0 each).
    pub factors: HashMap<String, f64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BatchScoreRequest {
    /// Map of DID → evidence.
    pub identities: HashMap<String, HashMap<String, f64>>,
    /// Optional custom threshold (0.0–1.0). Defaults to 0.3.
    pub threshold: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BatchScoreResponse {
    pub scores: Vec<ScoreResponse>,
    pub eligible_count: usize,
    pub total_count: usize,
}

// ── Handlers ────────────────────────────────────────────────────────

/// Score a single identity's sybil resistance.
#[utoipa::path(
    post, path = "/api/v1/sybil/score",
    tag = "governance",
    request_body = ScoreRequest,
    responses(
        (status = 200, description = "Sybil score computed", body = ScoreResponse)
    )
)]
pub async fn score_identity(Json(req): Json<ScoreRequest>) -> Json<ScoreResponse> {
    let mut scorer = SybilScorer::new();
    if let Some(t) = req.threshold {
        scorer.set_threshold(t);
    }

    let evidence = parse_evidence(&req.evidence);
    let result = scorer.score(&req.did, &evidence);

    Json(ScoreResponse {
        did: result.did,
        score: result.score,
        eligible: result.eligible,
        threshold: result.threshold,
        factors: result
            .factors
            .into_iter()
            .map(|(k, v)| (factor_name(&k), v))
            .collect(),
    })
}

/// Score multiple identities in a single request.
#[utoipa::path(
    post, path = "/api/v1/sybil/batch",
    tag = "governance",
    request_body = BatchScoreRequest,
    responses(
        (status = 200, description = "Batch sybil scores", body = BatchScoreResponse)
    )
)]
pub async fn score_batch(Json(req): Json<BatchScoreRequest>) -> Json<BatchScoreResponse> {
    let mut scorer = SybilScorer::new();
    if let Some(t) = req.threshold {
        scorer.set_threshold(t);
    }

    let scores: Vec<ScoreResponse> = req
        .identities
        .iter()
        .map(|(did, ev)| {
            let evidence = parse_evidence(ev);
            let result = scorer.score(did, &evidence);
            ScoreResponse {
                did: result.did,
                score: result.score,
                eligible: result.eligible,
                threshold: result.threshold,
                factors: result
                    .factors
                    .into_iter()
                    .map(|(k, v)| (factor_name(&k), v))
                    .collect(),
            }
        })
        .collect();

    let eligible_count = scores.iter().filter(|s| s.eligible).count();
    let total_count = scores.len();

    Json(BatchScoreResponse {
        scores,
        eligible_count,
        total_count,
    })
}

// ── Helpers ─────────────────────────────────────────────────────────

fn parse_factor(name: &str) -> Option<TrustFactor> {
    match name {
        "identity_age" => Some(TrustFactor::IdentityAge),
        "social_vouches" => Some(TrustFactor::SocialVouches),
        "on_chain_activity" => Some(TrustFactor::OnChainActivity),
        "stake_amount" => Some(TrustFactor::StakeAmount),
        "device_binding" => Some(TrustFactor::DeviceBinding),
        "credential_count" => Some(TrustFactor::CredentialCount),
        _ => None,
    }
}

fn factor_name(factor: &TrustFactor) -> String {
    match factor {
        TrustFactor::IdentityAge => "identity_age",
        TrustFactor::SocialVouches => "social_vouches",
        TrustFactor::OnChainActivity => "on_chain_activity",
        TrustFactor::StakeAmount => "stake_amount",
        TrustFactor::DeviceBinding => "device_binding",
        TrustFactor::CredentialCount => "credential_count",
    }
    .to_string()
}

fn parse_evidence(evidence: &HashMap<String, f64>) -> Vec<sybil::TrustEvidence> {
    evidence
        .iter()
        .filter_map(|(name, &value)| {
            parse_factor(name).map(|factor| sybil::evidence(factor, value))
        })
        .collect()
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

    fn json_request(method: &str, uri: &str, body: serde_json::Value) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap()
    }

    async fn parse_json(response: axum::http::Response<Body>) -> serde_json::Value {
        let body = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn score_identity_no_evidence() {
        let app = test_app().await;
        let req = serde_json::json!({
            "did": "did:key:z6MkEmpty",
            "evidence": {}
        });

        let resp = app
            .oneshot(json_request("POST", "/api/v1/sybil/score", req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let json = parse_json(resp).await;
        assert_eq!(json["did"], "did:key:z6MkEmpty");
        assert_eq!(json["score"], 0.0);
        assert_eq!(json["eligible"], false);
    }

    #[tokio::test]
    async fn score_identity_with_evidence() {
        let app = test_app().await;
        let req = serde_json::json!({
            "did": "did:key:z6MkAlice",
            "evidence": {
                "identity_age": 365.0,
                "social_vouches": 10.0,
                "on_chain_activity": 100.0,
                "stake_amount": 10000.0,
                "device_binding": 1.0,
                "credential_count": 5.0
            }
        });

        let resp = app
            .oneshot(json_request("POST", "/api/v1/sybil/score", req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let json = parse_json(resp).await;
        assert_eq!(json["did"], "did:key:z6MkAlice");
        let score = json["score"].as_f64().unwrap();
        assert!(score > 0.9, "expected high score, got {score}");
        assert_eq!(json["eligible"], true);
        assert!(json["factors"]["identity_age"].as_f64().unwrap() > 0.9);
    }

    #[tokio::test]
    async fn score_identity_custom_threshold() {
        let app = test_app().await;
        let req = serde_json::json!({
            "did": "did:key:z6MkBob",
            "evidence": {
                "identity_age": 30.0
            },
            "threshold": 0.01
        });

        let resp = app
            .oneshot(json_request("POST", "/api/v1/sybil/score", req))
            .await
            .unwrap();
        let json = parse_json(resp).await;

        // Low evidence but very low threshold
        assert_eq!(json["eligible"], true);
        assert_eq!(json["threshold"], 0.01);
    }

    #[tokio::test]
    async fn score_ignores_unknown_factors() {
        let app = test_app().await;
        let req = serde_json::json!({
            "did": "did:key:z6MkTest",
            "evidence": {
                "identity_age": 365.0,
                "unknown_factor": 999.0
            }
        });

        let resp = app
            .oneshot(json_request("POST", "/api/v1/sybil/score", req))
            .await
            .unwrap();
        let json = parse_json(resp).await;

        // Should only have the known factor
        assert!(json["factors"].get("identity_age").is_some());
        assert!(json["factors"].get("unknown_factor").is_none());
    }

    #[tokio::test]
    async fn batch_score() {
        let app = test_app().await;
        let req = serde_json::json!({
            "identities": {
                "did:key:z6MkAlice": {
                    "identity_age": 365.0,
                    "social_vouches": 10.0,
                    "on_chain_activity": 100.0
                },
                "did:key:z6MkBob": {}
            }
        });

        let resp = app
            .oneshot(json_request("POST", "/api/v1/sybil/batch", req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let json = parse_json(resp).await;
        assert_eq!(json["total_count"], 2);
        assert_eq!(json["eligible_count"], 1); // Alice eligible, Bob not
    }

    #[tokio::test]
    async fn batch_score_all_eligible() {
        let app = test_app().await;
        let req = serde_json::json!({
            "identities": {
                "did:key:z6MkA": {
                    "identity_age": 365.0,
                    "social_vouches": 10.0
                },
                "did:key:z6MkB": {
                    "on_chain_activity": 100.0,
                    "stake_amount": 10000.0
                }
            },
            "threshold": 0.1
        });

        let resp = app
            .oneshot(json_request("POST", "/api/v1/sybil/batch", req))
            .await
            .unwrap();
        let json = parse_json(resp).await;

        assert_eq!(json["total_count"], 2);
        assert_eq!(json["eligible_count"], 2);
    }
}
