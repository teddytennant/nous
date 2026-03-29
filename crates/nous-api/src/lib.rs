pub mod config;
pub mod error;
pub mod files;
pub mod graphql;
pub mod grpc;
pub mod middleware;
pub mod nostr;
pub mod openapi;
pub mod routes;
pub mod state;

pub use config::ApiConfig;
pub use graphql::NousSchema;

use axum::middleware as axum_mw;
use axum::routing::{delete, get, post};
use axum::Router;
use state::AppState;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;

async fn openapi_spec() -> axum::Json<utoipa::openapi::OpenApi> {
    axum::Json(openapi::NousApiDoc::openapi())
}

pub fn router(config: ApiConfig) -> Router {
    let state = AppState::new(config);

    let api = Router::new()
        .route("/health", get(routes::health))
        .route("/node", get(routes::node_info))
        .route("/feed", get(routes::get_feed))
        .route("/timeline", get(routes::get_timeline))
        .route("/events", post(routes::create_post))
        .route("/events/{event_id}", get(routes::get_event))
        .route("/events/{event_id}", delete(routes::delete_event))
        .route("/follow", post(routes::follow_user))
        .route("/unfollow", post(routes::unfollow_user))
        .route("/files", get(files::list_files))
        .route("/files", post(files::upload_file))
        .route("/files", delete(files::delete_file))
        .route("/files/stats", get(files::store_stats))
        .route("/files/latest", get(files::get_latest))
        .route("/files/history", get(files::get_history))
        .route("/files/{manifest_id}", get(files::get_file));

    Router::new()
        .nest("/api/v1", api)
        .route("/graphql", post(graphql::graphql_handler))
        .route("/api-docs/openapi.json", get(openapi_spec))
        .layer(axum_mw::from_fn(middleware::request_logger))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

pub async fn serve(config: ApiConfig) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", config.host, config.port);
    let grpc_addr = format!("{}:{}", config.host, config.port + 1);

    let state = AppState::new(config.clone());

    // gRPC server
    let grpc_handle = {
        let node_svc = grpc::NousNodeService::new(state.clone());
        let social_svc = grpc::NousSocialService::new(state.clone());
        let identity_svc = grpc::NousIdentityService::new(state.clone());

        let grpc_addr_parsed = grpc_addr.parse()?;
        tracing::info!(%grpc_addr, "nous gRPC server listening");

        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(grpc::pb::node_service_server::NodeServiceServer::new(
                    node_svc,
                ))
                .add_service(grpc::pb::social_service_server::SocialServiceServer::new(
                    social_svc,
                ))
                .add_service(
                    grpc::pb::identity_service_server::IdentityServiceServer::new(identity_svc),
                )
                .serve(grpc_addr_parsed)
                .await
        })
    };

    // REST + GraphQL server
    let app = router(config);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "nous API server listening");

    tokio::select! {
        result = axum::serve(listener, app) => result?,
        result = grpc_handle => result??,
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_creates() {
        let _router = router(ApiConfig::default());
    }
}
