pub mod ai;
pub mod config;
pub mod error;
pub mod files;
pub mod governance;
pub mod graphql;
pub mod grpc;
pub mod identity;
pub mod marketplace;
pub mod messaging;
pub mod middleware;
pub mod nostr;
pub mod openapi;
pub mod payments;
pub mod realtime;
pub mod routes;
pub mod state;

pub use config::ApiConfig;
pub use graphql::NousSchema;

use axum::Router;
use axum::middleware as axum_mw;
use axum::routing::{delete, get, post};
use state::AppState;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;

async fn openapi_spec() -> axum::Json<utoipa::openapi::OpenApi> {
    axum::Json(openapi::NousApiDoc::openapi())
}

/// Build a router with an in-memory database (tests / backwards compatibility).
pub fn router(config: ApiConfig) -> Router {
    let state = AppState::new(config);
    router_with_state(state)
}

/// Build a router backed by a pre-constructed AppState (allows real DB).
pub fn router_with_state(state: std::sync::Arc<AppState>) -> Router {
    let api = Router::new()
        // Node
        .route("/health", get(routes::health))
        .route("/node", get(routes::node_info))
        // Social
        .route("/feed", get(routes::get_feed))
        .route("/timeline", get(routes::get_timeline))
        .route("/events", post(routes::create_post))
        .route("/events/{event_id}", get(routes::get_event))
        .route("/events/{event_id}", delete(routes::delete_event))
        .route("/follow", post(routes::follow_user))
        .route("/unfollow", post(routes::unfollow_user))
        // Files
        .route("/files", get(files::list_files))
        .route("/files", post(files::upload_file))
        .route("/files", delete(files::delete_file))
        .route("/files/stats", get(files::store_stats))
        .route("/files/latest", get(files::get_latest))
        .route("/files/history", get(files::get_history))
        .route("/files/{manifest_id}", get(files::get_file))
        // Governance — DAOs
        .route("/daos", post(governance::create_dao))
        .route("/daos", get(governance::list_daos))
        .route("/daos/{dao_id}", get(governance::get_dao))
        .route("/daos/{dao_id}/members", post(governance::add_member))
        .route(
            "/daos/{dao_id}/members/{did}",
            delete(governance::remove_member),
        )
        // Governance — Proposals
        .route("/proposals", post(governance::submit_proposal))
        .route("/proposals", get(governance::list_proposals))
        .route("/proposals/{proposal_id}", get(governance::get_proposal))
        // Governance — Convenience (custodial signing)
        .route(
            "/daos/{dao_id}/proposals",
            post(governance::create_proposal),
        )
        .route(
            "/proposals/{proposal_id}/vote",
            post(governance::simple_vote),
        )
        // Governance — Voting
        .route("/votes", post(governance::cast_vote))
        .route("/votes/{proposal_id}", get(governance::get_tally))
        .route("/votes/private", post(governance::cast_private_vote))
        .route(
            "/votes/private/{proposal_id}",
            get(governance::get_private_tally),
        )
        // Governance — Delegation
        .route("/delegations", post(governance::create_delegation))
        .route("/delegations", get(governance::list_delegations))
        .route("/delegations/power", get(governance::get_effective_power))
        .route("/delegations/chain", get(governance::get_delegation_chain))
        .route(
            "/delegations/{delegation_id}/revoke",
            post(governance::revoke_delegation),
        )
        // Governance — Execution
        .route("/executions", post(governance::queue_execution))
        .route("/executions", get(governance::list_executions))
        .route("/executions/{execution_id}", get(governance::get_execution))
        .route(
            "/executions/{execution_id}/execute",
            post(governance::execute),
        )
        .route(
            "/executions/{execution_id}/cancel",
            post(governance::cancel_execution),
        )
        // Marketplace — Listings
        .route("/listings", post(marketplace::create_listing))
        .route("/listings", get(marketplace::search_listings))
        .route("/listings/{listing_id}", get(marketplace::get_listing))
        .route(
            "/listings/{listing_id}",
            delete(marketplace::cancel_listing),
        )
        .route(
            "/listings/{listing_id}/purchase",
            post(marketplace::purchase_listing),
        )
        // Marketplace — Orders
        .route("/orders", post(marketplace::create_order))
        .route("/orders", get(marketplace::list_orders))
        .route("/orders/{order_id}", get(marketplace::get_order))
        .route(
            "/orders/{order_id}/fund",
            post(marketplace::fund_order_escrow),
        )
        .route("/orders/{order_id}/ship", post(marketplace::ship_order))
        .route(
            "/orders/{order_id}/deliver",
            post(marketplace::confirm_delivery),
        )
        .route(
            "/orders/{order_id}/complete",
            post(marketplace::complete_order),
        )
        .route("/orders/{order_id}/cancel", post(marketplace::cancel_order))
        .route(
            "/orders/{order_id}/dispute",
            post(marketplace::dispute_order),
        )
        // Marketplace — Disputes
        .route("/disputes", post(marketplace::create_dispute))
        .route("/disputes", get(marketplace::list_disputes))
        .route("/disputes/{dispute_id}", get(marketplace::get_dispute))
        .route(
            "/disputes/{dispute_id}/evidence",
            post(marketplace::add_dispute_evidence),
        )
        .route(
            "/disputes/{dispute_id}/arbiter",
            post(marketplace::assign_dispute_arbiter),
        )
        .route(
            "/disputes/{dispute_id}/resolve-buyer",
            post(marketplace::resolve_dispute_buyer),
        )
        .route(
            "/disputes/{dispute_id}/resolve-seller",
            post(marketplace::resolve_dispute_seller),
        )
        .route(
            "/disputes/{dispute_id}/escalate",
            post(marketplace::escalate_dispute),
        )
        // Marketplace — Offers
        .route("/offers", post(marketplace::create_offer))
        .route("/offers", get(marketplace::list_offers))
        .route("/offers/{offer_id}", get(marketplace::get_offer))
        .route("/offers/{offer_id}/accept", post(marketplace::accept_offer))
        .route("/offers/{offer_id}/reject", post(marketplace::reject_offer))
        .route(
            "/offers/{offer_id}/counter",
            post(marketplace::counter_offer),
        )
        .route(
            "/offers/{offer_id}/withdraw",
            post(marketplace::withdraw_offer),
        )
        // Marketplace — Reviews
        .route("/reviews", post(marketplace::create_review))
        .route("/reviews", get(marketplace::list_reviews))
        .route(
            "/sellers/{seller_did}/rating",
            get(marketplace::get_seller_rating),
        )
        // Messaging — Channels
        .route("/channels", post(messaging::create_channel))
        .route("/channels", get(messaging::list_channels))
        .route("/channels/{channel_id}", get(messaging::get_channel))
        .route(
            "/channels/{channel_id}/members",
            post(messaging::add_channel_member),
        )
        .route(
            "/channels/{channel_id}/members/{did}",
            delete(messaging::remove_channel_member),
        )
        .route(
            "/channels/{channel_id}/messages",
            get(messaging::get_messages),
        )
        // Messaging — Messages
        .route("/messages", post(messaging::send_message))
        .route("/messages/{message_id}", delete(messaging::delete_message))
        // Identity
        .route("/identities", post(identity::create_identity))
        .route("/identities/{did}", get(identity::get_identity))
        .route("/identities/{did}/document", get(identity::get_document))
        .route(
            "/identities/{did}/credentials",
            get(identity::list_credentials),
        )
        .route(
            "/identities/{did}/credentials",
            post(identity::issue_credential),
        )
        .route(
            "/credentials/{credential_id}/verify",
            post(identity::verify_credential),
        )
        .route(
            "/identities/{did}/reputation",
            get(identity::get_reputation),
        )
        .route(
            "/identities/{did}/reputation",
            post(identity::add_reputation_event),
        )
        // Payments — Wallets
        .route("/wallets", post(payments::create_wallet))
        .route("/wallets/{did}", get(payments::get_wallet))
        .route("/wallets/{did}/credit", post(payments::credit_wallet))
        .route("/wallets/{did}/debit", post(payments::debit_wallet))
        .route(
            "/wallets/{did}/transactions",
            get(payments::get_transactions),
        )
        // Payments — Transfers
        .route("/transfers", post(payments::create_transfer))
        // Payments — Escrows
        .route("/escrows", post(payments::create_escrow))
        .route("/escrows/{escrow_id}", get(payments::get_escrow))
        .route(
            "/escrows/{escrow_id}/release",
            post(payments::release_escrow),
        )
        .route("/escrows/{escrow_id}/refund", post(payments::refund_escrow))
        .route(
            "/escrows/{escrow_id}/dispute",
            post(payments::dispute_escrow),
        )
        // Payments — Invoices
        .route("/invoices", post(payments::create_invoice))
        .route("/invoices", get(payments::list_invoices))
        .route("/invoices/{invoice_id}", get(payments::get_invoice))
        .route("/invoices/{invoice_id}/pay", post(payments::pay_invoice))
        .route(
            "/invoices/{invoice_id}/cancel",
            post(payments::cancel_invoice),
        )
        // AI — Agents
        .route("/agents", post(ai::create_agent))
        .route("/agents", get(ai::list_agents))
        .route("/agents/{agent_id}", get(ai::get_agent))
        .route("/agents/{agent_id}", delete(ai::delete_agent))
        // AI — Chat
        .route("/chat", post(ai::chat))
        // AI — Conversations
        .route("/conversations", get(ai::list_conversations))
        .route(
            "/conversations/{conversation_id}",
            get(ai::get_conversation),
        )
        .route(
            "/conversations/{conversation_id}",
            delete(ai::delete_conversation),
        )
        // Real-time
        .route("/ws", get(realtime::ws_handler))
        .route("/events", get(realtime::sse_handler));

    Router::new()
        .nest("/api/v1", api)
        .route("/graphql", post(graphql::graphql_handler))
        .route("/api-docs/openapi.json", get(openapi_spec))
        .layer(axum_mw::from_fn(middleware::request_logger))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Start the API server with an in-memory database (backwards compatible).
pub async fn serve(config: ApiConfig) -> Result<(), Box<dyn std::error::Error>> {
    let state = AppState::new(config.clone());
    serve_internal(config, state).await
}

/// Start the API server backed by a real SQLite database.
pub async fn serve_with_db(
    config: ApiConfig,
    db: nous_storage::Database,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = AppState::with_db(config.clone(), db);
    serve_internal(config, state).await
}

async fn serve_internal(
    config: ApiConfig,
    state: std::sync::Arc<AppState>,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", config.host, config.port);
    let grpc_addr = format!("{}:{}", config.host, config.port + 1);

    // gRPC server
    let grpc_handle = {
        let node_svc = grpc::NousNodeService::new(state.clone());
        let social_svc = grpc::NousSocialService::new(state.clone());
        let identity_svc = grpc::NousIdentityService::new(state.clone());
        let governance_svc = grpc::NousGovernanceService::new(state.clone());
        let marketplace_svc = grpc::NousMarketplaceService::new(state.clone());

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
                .add_service(
                    grpc::pb::governance_service_server::GovernanceServiceServer::new(
                        governance_svc,
                    ),
                )
                .add_service(
                    grpc::pb::marketplace_service_server::MarketplaceServiceServer::new(
                        marketplace_svc,
                    ),
                )
                .serve(grpc_addr_parsed)
                .await
        })
    };

    // REST + GraphQL server
    let app = router_with_state(state);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "nous API server listening");

    tokio::select! {
        result = axum::serve(listener, app) => result?,
        result = grpc_handle => result??,
    }

    Ok(())
}

/// Start the API server with an existing NousNode providing identity and
/// networking. Opens a second SQLite connection to the node's database so the
/// API layer has its own handle for concurrent writes (SQLite WAL supports this).
pub async fn serve_with_node(
    config: ApiConfig,
    node: &nous_node::NousNode,
) -> Result<(), Box<dyn std::error::Error>> {
    // Open a separate DB connection to the same file the node uses.
    let db_path = node.config().data_dir.join("nous.db");
    let db = nous_storage::Database::open(&db_path)?;
    let state = AppState::with_db(config.clone(), db);

    // Seed the API state with the node's identity.
    {
        let mut identities = state.identities.write().await;
        let did = node.did().to_string();
        tracing::info!(%did, "seeding API state with node identity");
        let id_copy = nous_identity::Identity::restore(&node.identity.export_signing_key())?;
        identities.insert(did, id_copy);
    }

    serve_internal(config, state).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_creates() {
        let _router = router(ApiConfig::default());
    }
}
