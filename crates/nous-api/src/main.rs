use nous_api::ApiConfig;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let api_config = ApiConfig::default();

    // Boot the NousNode orchestrator — opens DB, loads/creates identity, inits P2P.
    let node_config = nous_node::NodeConfig::default();
    let mut node = nous_node::NousNode::new(node_config)?;

    // Start the P2P networking layer (mDNS discovery, gossipsub, etc.)
    node.start().await?;

    tracing::info!(did = %node.did(), "nous node started");

    // Start the API server, seeded with the node's identity.
    let result = nous_api::serve_with_node(api_config, &node).await;

    // Clean shutdown.
    node.shutdown().await;

    result
}
