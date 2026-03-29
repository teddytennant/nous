use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::state::AppState;

pub mod pb {
    tonic::include_proto!("nous.v1");
}

use pb::node_service_server::NodeService;
use pb::social_service_server::SocialService;
use pb::identity_service_server::IdentityService;

// ---- Node Service ----

pub struct NousNodeService {
    _state: Arc<AppState>,
}

impl NousNodeService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { _state: state }
    }
}

#[tonic::async_trait]
impl NodeService for NousNodeService {
    async fn health(
        &self,
        _request: Request<pb::HealthRequest>,
    ) -> Result<Response<pb::HealthResponse>, Status> {
        Ok(Response::new(pb::HealthResponse {
            status: "ok".into(),
        }))
    }

    async fn node_info(
        &self,
        _request: Request<pb::NodeInfoRequest>,
    ) -> Result<Response<pb::NodeInfoResponse>, Status> {
        Ok(Response::new(pb::NodeInfoResponse {
            protocol: "nous/0.1".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            features: vec![
                "identity".into(),
                "messaging".into(),
                "governance".into(),
                "social".into(),
                "payments".into(),
                "ai".into(),
                "browser".into(),
                "storage".into(),
            ],
        }))
    }
}

// ---- Social Service ----

pub struct NousSocialService {
    state: Arc<AppState>,
}

impl NousSocialService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl SocialService for NousSocialService {
    async fn create_post(
        &self,
        request: Request<pb::CreatePostRequest>,
    ) -> Result<Response<pb::CreatePostResponse>, Status> {
        let req = request.into_inner();

        if req.content.is_empty() {
            return Err(Status::invalid_argument("content must not be empty"));
        }
        if req.content.len() > 10_000 {
            return Err(Status::invalid_argument("content exceeds 10,000 characters"));
        }

        let tags: Vec<nous_social::Tag> = req
            .hashtags
            .iter()
            .map(|t| nous_social::Tag::hashtag(t))
            .collect();

        let event = nous_social::SignedEvent::new(
            &req.author_did,
            nous_social::EventKind::TextNote,
            &req.content,
            tags,
        );
        let event_id = event.id.clone();

        let mut feed = self.state.feed.write().await;
        feed.insert(event);

        Ok(Response::new(pb::CreatePostResponse { event_id }))
    }

    async fn get_feed(
        &self,
        request: Request<pb::GetFeedRequest>,
    ) -> Result<Response<pb::GetFeedResponse>, Status> {
        let req = request.into_inner();
        let limit = if req.limit == 0 { 50 } else { req.limit as usize };
        let feed = self.state.feed.read().await;

        let events: Vec<&nous_social::SignedEvent> = if !req.author.is_empty() {
            feed.by_author(&req.author)
        } else if !req.hashtag.is_empty() {
            feed.by_hashtag(&req.hashtag)
        } else if req.kind > 0 {
            feed.by_kind(nous_social::EventKind::from(req.kind))
        } else {
            feed.latest(limit)
        };

        let event_messages: Vec<pb::EventMessage> = events
            .iter()
            .take(limit)
            .map(|e| event_to_proto(e))
            .collect();

        let count = event_messages.len() as u32;
        Ok(Response::new(pb::GetFeedResponse {
            events: event_messages,
            count,
        }))
    }

    async fn get_event(
        &self,
        request: Request<pb::GetEventRequest>,
    ) -> Result<Response<pb::EventMessage>, Status> {
        let req = request.into_inner();
        let feed = self.state.feed.read().await;

        // Search through all events for the matching ID
        let event = feed
            .latest(usize::MAX)
            .into_iter()
            .find(|e| e.id == req.event_id)
            .ok_or_else(|| Status::not_found("event not found"))?;

        Ok(Response::new(event_to_proto(event)))
    }

    async fn delete_event(
        &self,
        request: Request<pb::DeleteEventRequest>,
    ) -> Result<Response<pb::DeleteEventResponse>, Status> {
        let req = request.into_inner();
        let mut feed = self.state.feed.write().await;
        let deleted = feed.remove(&req.event_id);
        Ok(Response::new(pb::DeleteEventResponse { deleted }))
    }

    async fn follow(
        &self,
        request: Request<pb::FollowRequest>,
    ) -> Result<Response<pb::FollowResponse>, Status> {
        let req = request.into_inner();
        let mut graph = self.state.follow_graph.write().await;
        let success = graph.follow(&req.follower_did, &req.target_did);
        Ok(Response::new(pb::FollowResponse { success }))
    }

    async fn unfollow(
        &self,
        request: Request<pb::UnfollowRequest>,
    ) -> Result<Response<pb::UnfollowResponse>, Status> {
        let req = request.into_inner();
        let mut graph = self.state.follow_graph.write().await;
        let success = graph.unfollow(&req.follower_did, &req.target_did);
        Ok(Response::new(pb::UnfollowResponse { success }))
    }

    async fn get_timeline(
        &self,
        request: Request<pb::GetTimelineRequest>,
    ) -> Result<Response<pb::GetFeedResponse>, Status> {
        let req = request.into_inner();
        let limit = if req.limit == 0 { 50 } else { req.limit as usize };

        let graph = self.state.follow_graph.read().await;
        let following: Vec<String> = graph
            .following_of(&req.did)
            .iter()
            .map(|s| s.to_string())
            .collect();
        drop(graph);

        let feed = self.state.feed.read().await;
        let events: Vec<pb::EventMessage> = feed
            .timeline(&following, limit)
            .iter()
            .map(|e| event_to_proto(e))
            .collect();

        let count = events.len() as u32;
        Ok(Response::new(pb::GetFeedResponse {
            events,
            count,
        }))
    }
}

// ---- Identity Service ----

pub struct NousIdentityService {
    _state: Arc<AppState>,
}

impl NousIdentityService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { _state: state }
    }
}

#[tonic::async_trait]
impl IdentityService for NousIdentityService {
    async fn resolve_document(
        &self,
        request: Request<pb::ResolveDocumentRequest>,
    ) -> Result<Response<pb::ResolveDocumentResponse>, Status> {
        let req = request.into_inner();

        // Validate DID format
        if !req.did.starts_with("did:key:z") {
            return Err(Status::invalid_argument("invalid DID format"));
        }

        // For now, return a minimal document. In production this would
        // resolve from the DHT or local store.
        let doc_json = serde_json::json!({
            "@context": ["https://www.w3.org/ns/did/v1"],
            "id": req.did,
        })
        .to_string();

        Ok(Response::new(pb::ResolveDocumentResponse { document_json: doc_json }))
    }
}

fn event_to_proto(event: &nous_social::SignedEvent) -> pb::EventMessage {
    pb::EventMessage {
        id: event.id.clone(),
        pubkey: event.pubkey.clone(),
        created_at: event.created_at.timestamp(),
        kind: event.kind.as_u32(),
        content: event.content.clone(),
        hashtags: event.hashtags().iter().map(|s| s.to_string()).collect(),
        referenced_events: event
            .referenced_events()
            .iter()
            .map(|s| s.to_string())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiConfig;

    fn test_state() -> Arc<AppState> {
        AppState::new(ApiConfig::default())
    }

    #[tokio::test]
    async fn grpc_health() {
        let svc = NousNodeService::new(test_state());
        let resp = svc
            .health(Request::new(pb::HealthRequest {}))
            .await
            .unwrap();
        assert_eq!(resp.into_inner().status, "ok");
    }

    #[tokio::test]
    async fn grpc_node_info() {
        let svc = NousNodeService::new(test_state());
        let resp = svc
            .node_info(Request::new(pb::NodeInfoRequest {}))
            .await
            .unwrap();
        let info = resp.into_inner();
        assert_eq!(info.protocol, "nous/0.1");
        assert!(!info.features.is_empty());
    }

    #[tokio::test]
    async fn grpc_create_post() {
        let state = test_state();
        let svc = NousSocialService::new(state.clone());

        let resp = svc
            .create_post(Request::new(pb::CreatePostRequest {
                author_did: "did:key:ztest".into(),
                content: "hello grpc".into(),
                hashtags: vec!["nous".into()],
            }))
            .await
            .unwrap();

        assert!(!resp.into_inner().event_id.is_empty());

        let feed = state.feed.read().await;
        assert_eq!(feed.len(), 1);
    }

    #[tokio::test]
    async fn grpc_create_post_empty_content() {
        let svc = NousSocialService::new(test_state());
        let result = svc
            .create_post(Request::new(pb::CreatePostRequest {
                author_did: "did:key:ztest".into(),
                content: "".into(),
                hashtags: vec![],
            }))
            .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn grpc_get_feed() {
        let state = test_state();
        let svc = NousSocialService::new(state.clone());

        // Insert a post first
        svc.create_post(Request::new(pb::CreatePostRequest {
            author_did: "did:key:ztest".into(),
            content: "feed item".into(),
            hashtags: vec![],
        }))
        .await
        .unwrap();

        let resp = svc
            .get_feed(Request::new(pb::GetFeedRequest {
                author: "".into(),
                kind: 0,
                hashtag: "".into(),
                limit: 10,
            }))
            .await
            .unwrap();

        let feed = resp.into_inner();
        assert_eq!(feed.count, 1);
        assert_eq!(feed.events[0].content, "feed item");
    }

    #[tokio::test]
    async fn grpc_get_feed_by_author() {
        let state = test_state();
        let svc = NousSocialService::new(state.clone());

        svc.create_post(Request::new(pb::CreatePostRequest {
            author_did: "did:key:zalice".into(),
            content: "alice post".into(),
            hashtags: vec![],
        }))
        .await
        .unwrap();

        svc.create_post(Request::new(pb::CreatePostRequest {
            author_did: "did:key:zbob".into(),
            content: "bob post".into(),
            hashtags: vec![],
        }))
        .await
        .unwrap();

        let resp = svc
            .get_feed(Request::new(pb::GetFeedRequest {
                author: "did:key:zalice".into(),
                kind: 0,
                hashtag: "".into(),
                limit: 10,
            }))
            .await
            .unwrap();

        let feed = resp.into_inner();
        assert_eq!(feed.count, 1);
        assert_eq!(feed.events[0].pubkey, "did:key:zalice");
    }

    #[tokio::test]
    async fn grpc_delete_event() {
        let state = test_state();
        let svc = NousSocialService::new(state.clone());

        let resp = svc
            .create_post(Request::new(pb::CreatePostRequest {
                author_did: "did:key:ztest".into(),
                content: "to delete".into(),
                hashtags: vec![],
            }))
            .await
            .unwrap();
        let event_id = resp.into_inner().event_id;

        let del = svc
            .delete_event(Request::new(pb::DeleteEventRequest {
                event_id: event_id.clone(),
            }))
            .await
            .unwrap();
        assert!(del.into_inner().deleted);

        let feed = state.feed.read().await;
        assert_eq!(feed.len(), 0);
    }

    #[tokio::test]
    async fn grpc_follow_unfollow() {
        let state = test_state();
        let svc = NousSocialService::new(state.clone());

        let resp = svc
            .follow(Request::new(pb::FollowRequest {
                follower_did: "did:key:zalice".into(),
                target_did: "did:key:zbob".into(),
            }))
            .await
            .unwrap();
        assert!(resp.into_inner().success);

        let graph = state.follow_graph.read().await;
        assert!(graph.is_following("did:key:zalice", "did:key:zbob"));
        drop(graph);

        let resp = svc
            .unfollow(Request::new(pb::UnfollowRequest {
                follower_did: "did:key:zalice".into(),
                target_did: "did:key:zbob".into(),
            }))
            .await
            .unwrap();
        assert!(resp.into_inner().success);
    }

    #[tokio::test]
    async fn grpc_get_event_not_found() {
        let svc = NousSocialService::new(test_state());
        let result = svc
            .get_event(Request::new(pb::GetEventRequest {
                event_id: "nonexistent".into(),
            }))
            .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn grpc_resolve_document() {
        let svc = NousIdentityService::new(test_state());
        let resp = svc
            .resolve_document(Request::new(pb::ResolveDocumentRequest {
                did: "did:key:ztest123".into(),
            }))
            .await
            .unwrap();
        let doc = resp.into_inner().document_json;
        assert!(doc.contains("did:key:ztest123"));
    }

    #[tokio::test]
    async fn grpc_resolve_document_invalid_did() {
        let svc = NousIdentityService::new(test_state());
        let result = svc
            .resolve_document(Request::new(pb::ResolveDocumentRequest {
                did: "invalid".into(),
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn grpc_timeline() {
        let state = test_state();
        let svc = NousSocialService::new(state.clone());

        // Create posts from two users
        svc.create_post(Request::new(pb::CreatePostRequest {
            author_did: "did:key:zalice".into(),
            content: "alice post".into(),
            hashtags: vec![],
        }))
        .await
        .unwrap();

        svc.create_post(Request::new(pb::CreatePostRequest {
            author_did: "did:key:zbob".into(),
            content: "bob post".into(),
            hashtags: vec![],
        }))
        .await
        .unwrap();

        // Follow alice only
        svc.follow(Request::new(pb::FollowRequest {
            follower_did: "did:key:zme".into(),
            target_did: "did:key:zalice".into(),
        }))
        .await
        .unwrap();

        let resp = svc
            .get_timeline(Request::new(pb::GetTimelineRequest {
                did: "did:key:zme".into(),
                limit: 10,
            }))
            .await
            .unwrap();

        let timeline = resp.into_inner();
        assert_eq!(timeline.count, 1);
        assert_eq!(timeline.events[0].pubkey, "did:key:zalice");
    }
}
