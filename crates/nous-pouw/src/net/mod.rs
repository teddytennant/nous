//! libp2p gossipsub binding for the [`crate::network::Network`] trait.
//!
//! Spawns a tokio-driven libp2p swarm in the background and exposes a
//! handle ([`GossipNetwork`]) that the consensus engine can publish to and
//! drain inbound events from.
//!
//! The swarm uses TCP + noise + yamux + gossipsub, optional mDNS for LAN
//! discovery, and identify for peer-id exchange. The local peer identity is
//! derived from the same `ed25519_dalek::SigningKey` the chain uses, so peer
//! ids and worker ids share their public key bytes.

pub use gossip::{GossipNetwork, GossipNetworkConfig};

mod gossip;
mod task;
