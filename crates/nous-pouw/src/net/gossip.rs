//! Public [`GossipNetwork`] handle that satisfies the [`crate::Network`] trait
//! over a real libp2p gossipsub swarm running in a tokio task.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use ed25519_dalek::SigningKey;
use parking_lot::Mutex;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tracing::warn;

use crate::network::{Network, NetworkEvent, Topic};
use crate::state::WorkerId;

use super::task::{self, Outbound, PouwBehaviour};

/// Configuration for [`GossipNetwork::spawn`].
#[derive(Debug, Clone)]
pub struct GossipNetworkConfig {
    /// Listen multiaddr, e.g. `"/ip4/0.0.0.0/tcp/0"`.
    pub listen_addr: String,
    /// Optional bootstrap peers as multiaddrs (the swarm dials these on
    /// startup).
    pub bootstrap: Vec<String>,
    /// Enable mDNS for LAN peer discovery.
    pub mdns: bool,
}

impl Default for GossipNetworkConfig {
    fn default() -> Self {
        Self {
            listen_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
            bootstrap: Vec::new(),
            mdns: false,
        }
    }
}

/// Handle to a running libp2p gossipsub network. Implements
/// [`crate::Network`]: `publish` enqueues an outbound gossip message,
/// `drain` returns and clears all inbound messages received since the last
/// call.
///
/// Dropping this handle aborts the background swarm task.
pub struct GossipNetwork {
    outbound: mpsc::UnboundedSender<Outbound>,
    inbound: Arc<Mutex<VecDeque<NetworkEvent>>>,
    local_addr: String,
    task: Option<JoinHandle<()>>,
}

impl GossipNetwork {
    /// Spawn a libp2p swarm bound to the given keypair and config.
    ///
    /// Returns once the swarm is listening (i.e. [`Self::local_addr`] is
    /// populated). The actual event loop runs in a background tokio task.
    pub async fn spawn(
        signing_key: SigningKey,
        cfg: GossipNetworkConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Convert ed25519-dalek SigningKey -> libp2p Keypair (same bytes).
        let secret_bytes = signing_key.to_bytes();
        let local_key = libp2p::identity::Keypair::ed25519_from_bytes(secret_bytes)
            .map_err(|e| format!("ed25519 keypair: {e}"))?;
        let local_peer_id = libp2p::PeerId::from(local_key.public());

        let mdns_enabled = cfg.mdns;
        let listen_addr_in = cfg.listen_addr.clone();
        let bootstrap = cfg.bootstrap.clone();

        let local_key_for_builder = local_key.clone();
        let swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default().nodelay(true),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )?
            .with_behaviour(|_key| {
                PouwBehaviour::new(&local_key_for_builder, local_peer_id, mdns_enabled)
                    .expect("pouw behaviour build")
            })?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(300)))
            .build();

        let (out_tx, out_rx) = mpsc::unbounded_channel::<Outbound>();
        let inbound = Arc::new(Mutex::new(VecDeque::<NetworkEvent>::new()));
        let inbound_for_task = inbound.clone();
        let (addr_tx, addr_rx) = oneshot::channel::<String>();

        let task = tokio::spawn(task::run(
            swarm,
            out_rx,
            inbound_for_task,
            listen_addr_in,
            bootstrap,
            Some(addr_tx),
        ));

        // Wait for the first NewListenAddr (or fail loudly after a short
        // timeout — the listen_on call is synchronous-ish).
        let local_addr =
            match tokio::time::timeout(Duration::from_secs(5), addr_rx).await {
                Ok(Ok(addr)) => addr,
                Ok(Err(_)) => return Err("swarm task ended before listening".into()),
                Err(_) => return Err("timed out waiting for listen address".into()),
            };

        Ok(Self {
            outbound: out_tx,
            inbound,
            local_addr,
            task: Some(task),
        })
    }

    /// Multiaddr the local node is listening on (populated after [`spawn`]).
    /// For an ephemeral `/tcp/0` listen, this contains the resolved port.
    pub fn local_addr(&self) -> String {
        self.local_addr.clone()
    }
}

impl Network for GossipNetwork {
    fn publish(&self, topic: Topic, from: WorkerId, payload: Vec<u8>) {
        if let Err(e) = self.outbound.send(Outbound {
            topic,
            from,
            payload,
        }) {
            warn!(%e, "GossipNetwork publish failed: swarm task gone");
        }
    }

    fn drain(&self) -> Vec<NetworkEvent> {
        let mut buf = self.inbound.lock();
        let out = buf.drain(..).collect();
        out
    }
}

impl Drop for GossipNetwork {
    fn drop(&mut self) {
        if let Some(handle) = self.task.take() {
            handle.abort();
        }
    }
}
