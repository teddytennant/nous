pub mod config;
pub mod error;
pub mod graphql;
pub mod middleware;
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
        .route("/unfollow", post(routes::unfollow_user));

    Router::new()
        .nest("/api/v1", api)
        .route("/graphql", post(graphql::graphql_handler))
        .layer(axum_mw::from_fn(middleware::request_logger))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

pub async fn serve(config: ApiConfig) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", config.host, config.port);
    let app = router(config);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "nous API server listening");
    axum::serve(listener, app).await?;
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
