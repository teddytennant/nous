//! Internal: the libp2p swarm task.
//!
//! Owns the swarm and translates between the public mpsc / event-buffer API
//! and the libp2p `SwarmEvent` stream.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use libp2p::gossipsub::{self, IdentTopic, MessageAuthenticity, ValidationMode};
use libp2p::swarm::SwarmEvent;
use libp2p::{Multiaddr, PeerId, Swarm, identify, mdns};
use parking_lot::Mutex;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, warn};

use crate::network::{NetworkEvent, Topic};
use crate::state::WorkerId;

/// All PoUW topics the swarm subscribes to on startup.
pub(crate) const ALL_TOPICS: &[Topic] = &[
    Topic::Jobs,
    Topic::ReceiptCommits,
    Topic::ReceiptReveals,
    Topic::Blocks,
    Topic::Votes,
    Topic::Slashes,
];

/// Stable wire-name mapping. Keep in sync with [`topic_from_wire`].
pub(crate) fn topic_to_wire(t: Topic) -> &'static str {
    match t {
        Topic::Jobs => "pouw/jobs",
        Topic::ReceiptCommits => "pouw/receipt-commits",
        Topic::ReceiptReveals => "pouw/receipt-reveals",
        Topic::Blocks => "pouw/blocks",
        Topic::Votes => "pouw/votes",
        Topic::Slashes => "pouw/slashes",
    }
}

pub(crate) fn topic_from_wire(s: &str) -> Option<Topic> {
    match s {
        "pouw/jobs" => Some(Topic::Jobs),
        "pouw/receipt-commits" => Some(Topic::ReceiptCommits),
        "pouw/receipt-reveals" => Some(Topic::ReceiptReveals),
        "pouw/blocks" => Some(Topic::Blocks),
        "pouw/votes" => Some(Topic::Votes),
        "pouw/slashes" => Some(Topic::Slashes),
        _ => None,
    }
}

/// One outbound publish request from the public API.
pub(crate) struct Outbound {
    pub topic: Topic,
    pub from: WorkerId,
    pub payload: Vec<u8>,
}

/// The combined libp2p behaviour driven by the task.
#[derive(libp2p::swarm::NetworkBehaviour)]
pub(crate) struct PouwBehaviour {
    pub(crate) gossipsub: gossipsub::Behaviour,
    pub(crate) identify: identify::Behaviour,
    pub(crate) mdns: libp2p::swarm::behaviour::toggle::Toggle<mdns::tokio::Behaviour>,
}

impl PouwBehaviour {
    pub(crate) fn new(
        local_key: &libp2p::identity::Keypair,
        local_peer_id: PeerId,
        enable_mdns: bool,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let gossipsub_cfg = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_millis(250))
            .validation_mode(ValidationMode::Strict)
            .max_transmit_size(1024 * 1024)
            .build()
            .map_err(|e| format!("gossipsub config: {e}"))?;

        let gossipsub = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_cfg,
        )
        .map_err(|e| format!("gossipsub behaviour: {e}"))?;

        let identify = identify::Behaviour::new(identify::Config::new(
            "/pouw/id/1.0.0".to_string(),
            local_key.public(),
        ));

        let mdns_inner = if enable_mdns {
            Some(mdns::tokio::Behaviour::new(
                mdns::Config::default(),
                local_peer_id,
            )?)
        } else {
            None
        };
        let mdns = libp2p::swarm::behaviour::toggle::Toggle::from(mdns_inner);

        Ok(Self {
            gossipsub,
            identify,
            mdns,
        })
    }
}

/// Wire envelope for one PoUW gossip message: prepend the 32-byte sender key
/// so listeners can recover [`NetworkEvent::from`] without parsing the
/// inner payload.
pub(crate) fn encode_wire(from: WorkerId, payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(32 + payload.len());
    buf.extend_from_slice(&from.0);
    buf.extend_from_slice(payload);
    buf
}

pub(crate) fn decode_wire(bytes: &[u8]) -> Option<(WorkerId, Vec<u8>)> {
    if bytes.len() < 32 {
        return None;
    }
    let mut id = [0u8; 32];
    id.copy_from_slice(&bytes[..32]);
    Some((WorkerId(id), bytes[32..].to_vec()))
}

/// Run the swarm event loop until either the outbound channel closes or a
/// shutdown signal arrives. Returning lets the spawned tokio task exit
/// cleanly so `Drop` on `GossipNetwork` doesn't leak the swarm.
pub(crate) async fn run(
    mut swarm: Swarm<PouwBehaviour>,
    mut outbound: mpsc::UnboundedReceiver<Outbound>,
    inbound: Arc<Mutex<VecDeque<NetworkEvent>>>,
    listen_addr: String,
    bootstrap: Vec<String>,
    listen_addr_tx: Option<oneshot::Sender<String>>,
) {
    // Subscribe to every PoUW topic.
    for t in ALL_TOPICS {
        let topic = IdentTopic::new(topic_to_wire(*t));
        if let Err(e) = swarm.behaviour_mut().gossipsub.subscribe(&topic) {
            warn!(?t, %e, "subscribe failed");
        }
    }

    // Begin listening.
    let parsed_listen: Multiaddr = match listen_addr.parse() {
        Ok(a) => a,
        Err(e) => {
            warn!(%listen_addr, %e, "invalid listen multiaddr; aborting");
            return;
        }
    };
    if let Err(e) = swarm.listen_on(parsed_listen) {
        warn!(%e, "listen_on failed");
        return;
    }

    // Dial bootstrap peers.
    for addr in &bootstrap {
        match addr.parse::<Multiaddr>() {
            Ok(ma) => {
                if let Err(e) = swarm.dial(ma) {
                    warn!(%addr, %e, "bootstrap dial failed");
                }
            }
            Err(e) => warn!(%addr, %e, "bad bootstrap multiaddr"),
        }
    }

    let mut listen_addr_tx = listen_addr_tx;

    loop {
        tokio::select! {
            // Outbound publish requests.
            req = outbound.recv() => {
                let Some(req) = req else {
                    // Sender dropped — shut down.
                    debug!("outbound channel closed; shutting down swarm task");
                    return;
                };
                let topic = IdentTopic::new(topic_to_wire(req.topic));
                let bytes = encode_wire(req.from, &req.payload);
                if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, bytes) {
                    // InsufficientPeers is normal during mesh warmup — log at debug.
                    debug!(?req.topic, %e, "publish error (likely no peers yet)");
                }
            }

            // Swarm events.
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        info!(%address, "pouw libp2p listening");
                        if let Some(tx) = listen_addr_tx.take() {
                            let _ = tx.send(address.to_string());
                        }
                    }
                    SwarmEvent::Behaviour(PouwBehaviourEvent::Gossipsub(
                        gossipsub::Event::Message { message, .. },
                    )) => {
                        let topic = match topic_from_wire(message.topic.as_str()) {
                            Some(t) => t,
                            None => {
                                debug!(topic = %message.topic, "ignoring message on unknown topic");
                                continue;
                            }
                        };
                        let (from, payload) = match decode_wire(&message.data) {
                            Some(x) => x,
                            None => {
                                debug!("ignoring malformed wire message");
                                continue;
                            }
                        };
                        inbound.lock().push_back(NetworkEvent {
                            topic,
                            from: Some(from),
                            payload,
                        });
                    }
                    SwarmEvent::Behaviour(PouwBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                        for (peer_id, addr) in peers {
                            debug!(%peer_id, %addr, "mdns discovered");
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        }
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        debug!(%peer_id, "pouw peer connected");
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                    SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        debug!(%peer_id, "pouw peer disconnected");
                    }
                    _ => {}
                }
            }
        }
    }
}
