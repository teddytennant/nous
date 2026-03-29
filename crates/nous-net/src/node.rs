use futures::StreamExt;
use libp2p::gossipsub;
use libp2p::identify;
use libp2p::kad;
use libp2p::mdns;
use libp2p::swarm::SwarmEvent;
use libp2p::{Multiaddr, PeerId, Swarm};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::behaviour::{NousBehaviour, NousBehaviourEvent};
use crate::connection_manager::{ConnectionManager, Direction};
use crate::events::{NodeEvent, WireMessage};
use crate::peer_store::PeerStore;
use crate::rate_limit::RateLimiter;
use crate::signing::verify_message;
use crate::topics::NousTopic;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub listen_addresses: Vec<String>,
    pub bootstrap_peers: Vec<String>,
    pub enable_mdns: bool,
    pub enable_relay: bool,
    pub max_connections: usize,
    /// Max known peers in the peer store.
    pub max_known_peers: usize,
    /// Reject unsigned gossip messages.
    pub require_signatures: bool,
    /// Rate limit: burst tokens per peer.
    pub rate_limit_burst: u32,
    /// Rate limit: tokens refilled per second.
    pub rate_limit_per_second: u32,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            listen_addresses: vec!["/ip4/0.0.0.0/tcp/0".to_string()],
            bootstrap_peers: Vec::new(),
            enable_mdns: true,
            enable_relay: true,
            max_connections: 128,
            max_known_peers: 1024,
            require_signatures: false,
            rate_limit_burst: 60,
            rate_limit_per_second: 10,
        }
    }
}

pub struct NousNode {
    swarm: Swarm<NousBehaviour>,
    event_tx: mpsc::Sender<NodeEvent>,
    connected_peers: HashSet<PeerId>,
    subscribed_topics: HashSet<NousTopic>,
    local_peer_id: PeerId,
    rate_limiter: RateLimiter,
    peer_store: PeerStore,
    connection_manager: ConnectionManager,
    require_signatures: bool,
    bootstrap_peers: Vec<String>,
}

impl NousNode {
    pub fn new(
        config: &NodeConfig,
    ) -> Result<(Self, mpsc::Receiver<NodeEvent>), Box<dyn std::error::Error>> {
        let local_key = libp2p::identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        info!(%local_peer_id, "initializing nous node");

        let swarm = libp2p::SwarmBuilder::with_existing_identity(local_key.clone())
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default().nodelay(true),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )?
            .with_relay_client(libp2p::noise::Config::new, libp2p::yamux::Config::default)?
            .with_behaviour(|_key, relay_behaviour| {
                NousBehaviour::new(local_peer_id, &local_key, relay_behaviour)
                    .expect("behaviour creation should not fail")
            })?
            .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(300)))
            .build();

        let (event_tx, event_rx) = mpsc::channel(1024);

        let rate_limiter = RateLimiter::new(
            config.rate_limit_burst,
            config.rate_limit_per_second,
            Duration::from_secs(1),
        );

        let peer_store = PeerStore::new(config.max_known_peers);
        let connection_manager = ConnectionManager::new(config.max_connections);

        Ok((
            Self {
                swarm,
                event_tx,
                connected_peers: HashSet::new(),
                subscribed_topics: HashSet::new(),
                local_peer_id,
                rate_limiter,
                peer_store,
                connection_manager,
                require_signatures: config.require_signatures,
                bootstrap_peers: config.bootstrap_peers.clone(),
            },
            event_rx,
        ))
    }

    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    pub fn connected_peers(&self) -> &HashSet<PeerId> {
        &self.connected_peers
    }

    pub fn subscribed_topics(&self) -> &HashSet<NousTopic> {
        &self.subscribed_topics
    }

    pub fn peer_store(&self) -> &PeerStore {
        &self.peer_store
    }

    pub fn connection_manager(&self) -> &ConnectionManager {
        &self.connection_manager
    }

    pub fn listen_on(&mut self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let multiaddr: Multiaddr = addr.parse()?;
        self.swarm.listen_on(multiaddr)?;
        Ok(())
    }

    pub fn dial(&mut self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let multiaddr: Multiaddr = addr.parse()?;
        self.swarm.dial(multiaddr)?;
        Ok(())
    }

    pub fn subscribe(&mut self, topic: &NousTopic) -> Result<(), Box<dyn std::error::Error>> {
        self.swarm.behaviour_mut().subscribe(topic)?;
        self.subscribed_topics.insert(topic.clone());
        info!(%topic, "subscribed to topic");
        Ok(())
    }

    pub fn subscribe_all_default(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for topic in NousTopic::all_default() {
            self.subscribe(&topic)?;
        }
        Ok(())
    }

    pub fn publish(
        &mut self,
        message: &WireMessage,
    ) -> Result<gossipsub::MessageId, Box<dyn std::error::Error>> {
        if self.require_signatures && message.signature.is_empty() {
            return Err("message must be signed before publishing".into());
        }

        let data = message.encode()?;
        let msg_id = self.swarm.behaviour_mut().publish(&message.topic, data)?;
        debug!(topic = %message.topic, "published message");
        Ok(msg_id)
    }

    pub fn put_record(
        &mut self,
        key: &str,
        value: Vec<u8>,
    ) -> Result<kad::QueryId, Box<dyn std::error::Error>> {
        let record = kad::Record {
            key: kad::RecordKey::new(&key),
            value,
            publisher: Some(self.local_peer_id),
            expires: None,
        };
        let query_id = self
            .swarm
            .behaviour_mut()
            .kademlia
            .put_record(record, kad::Quorum::One)?;
        Ok(query_id)
    }

    pub fn get_record(&mut self, key: &str) -> kad::QueryId {
        self.swarm
            .behaviour_mut()
            .kademlia
            .get_record(kad::RecordKey::new(&key))
    }

    pub fn add_peer(&mut self, peer_id: PeerId, addr: Multiaddr) {
        self.peer_store.record_peer(peer_id, addr.clone());
        self.swarm
            .behaviour_mut()
            .kademlia
            .add_address(&peer_id, addr);
    }

    /// Connect to configured bootstrap peers.
    pub fn bootstrap(&mut self) -> usize {
        let peers = self.bootstrap_peers.clone();
        let mut connected = 0;
        for addr_str in &peers {
            match addr_str.parse::<Multiaddr>() {
                Ok(addr) => {
                    if let Err(e) = self.swarm.dial(addr) {
                        warn!(%addr_str, %e, "failed to dial bootstrap peer");
                    } else {
                        info!(%addr_str, "dialing bootstrap peer");
                        connected += 1;
                    }
                }
                Err(e) => {
                    warn!(%addr_str, %e, "invalid bootstrap peer address");
                }
            }
        }
        connected
    }

    /// Export the peer store for persistence.
    pub fn export_peers(&self) -> Vec<crate::peer_store::PeerRecord> {
        self.peer_store.export()
    }

    /// Import previously persisted peer records.
    pub fn import_peers(&mut self, records: Vec<crate::peer_store::PeerRecord>) {
        self.peer_store.import(records);
    }

    pub async fn run(&mut self) {
        // Bootstrap on startup.
        let bootstrapped = self.bootstrap();
        if bootstrapped > 0 {
            info!(count = bootstrapped, "bootstrap dials initiated");
        }

        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::Behaviour(event) => self.handle_behaviour_event(event).await,
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!(%address, "listening on");
                    let _ = self.event_tx.send(NodeEvent::ListeningOn(address)).await;
                }
                SwarmEvent::ConnectionEstablished {
                    peer_id, endpoint, ..
                } => {
                    let direction = if endpoint.is_dialer() {
                        Direction::Outbound
                    } else {
                        Direction::Inbound
                    };

                    // Check connection manager limits.
                    match self.connection_manager.register(peer_id, direction) {
                        Ok(()) => {}
                        Err(Some(evict_peer)) => {
                            warn!(%evict_peer, "evicting low-score peer for capacity");
                            self.connection_manager.disconnect(&evict_peer);
                            // Still register the new peer.
                            let _ = self.connection_manager.register(peer_id, direction);
                        }
                        Err(None) => {
                            warn!(%peer_id, "at max connections, all protected");
                        }
                    }

                    // Record in peer store.
                    let addr = endpoint.get_remote_address().clone();
                    self.peer_store.record_peer(peer_id, addr);
                    self.peer_store.record_connection(&peer_id);

                    self.connected_peers.insert(peer_id);
                    info!(%peer_id, peers = self.connected_peers.len(), "peer connected");
                    let _ = self.event_tx.send(NodeEvent::PeerConnected(peer_id)).await;
                }
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    self.connected_peers.remove(&peer_id);
                    self.connection_manager.disconnect(&peer_id);
                    self.rate_limiter.remove_peer(&peer_id);
                    info!(%peer_id, peers = self.connected_peers.len(), "peer disconnected");
                    let _ = self
                        .event_tx
                        .send(NodeEvent::PeerDisconnected(peer_id))
                        .await;
                }
                SwarmEvent::OutgoingConnectionError {
                    error, peer_id, ..
                } => {
                    if let Some(pid) = peer_id {
                        self.peer_store.record_dial_failure(&pid);
                    }
                    warn!(%error, "outgoing connection failed");
                }
                SwarmEvent::IncomingConnectionError { error, .. } => {
                    warn!(%error, "incoming connection failed");
                }
                _ => {}
            }
        }
    }

    async fn handle_behaviour_event(&mut self, event: NousBehaviourEvent) {
        match event {
            NousBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                propagation_source,
                message,
                ..
            }) => {
                // Rate limiting.
                if !self.rate_limiter.check(&propagation_source) {
                    debug!(%propagation_source, "rate limited — dropping message");
                    return;
                }

                let topic = NousTopic::from_topic_hash(&message.topic)
                    .unwrap_or(NousTopic::Custom(message.topic.to_string()));

                // Attempt to decode and verify signature if required.
                if self.require_signatures {
                    match WireMessage::decode(&message.data) {
                        Ok(wire_msg) => {
                            if let Err(e) = verify_message(&wire_msg) {
                                warn!(
                                    %propagation_source,
                                    %e,
                                    "dropping message with invalid signature"
                                );
                                self.peer_store.adjust_score(&propagation_source, -5);
                                return;
                            }
                            // Reward good behaviour.
                            self.peer_store.adjust_score(&propagation_source, 1);
                        }
                        Err(_) => {
                            // Non-WireMessage format — let through for backwards compat.
                        }
                    }
                }

                // Track traffic.
                self.connection_manager.record_traffic(
                    &propagation_source,
                    0,
                    message.data.len() as u64,
                );

                debug!(
                    %propagation_source,
                    %topic,
                    bytes = message.data.len(),
                    "gossipsub message received"
                );

                let _ = self
                    .event_tx
                    .send(NodeEvent::MessageReceived {
                        source: message.source,
                        topic,
                        data: message.data,
                    })
                    .await;
            }
            NousBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed { peer_id, topic }) => {
                if let Some(t) = NousTopic::from_topic_hash(&topic) {
                    let _ = self
                        .event_tx
                        .send(NodeEvent::Subscribed {
                            peer: peer_id,
                            topic: t,
                        })
                        .await;
                }
            }
            NousBehaviourEvent::Gossipsub(gossipsub::Event::Unsubscribed { peer_id, topic }) => {
                if let Some(t) = NousTopic::from_topic_hash(&topic) {
                    let _ = self
                        .event_tx
                        .send(NodeEvent::Unsubscribed {
                            peer: peer_id,
                            topic: t,
                        })
                        .await;
                }
            }
            NousBehaviourEvent::Mdns(mdns::Event::Discovered(peers)) => {
                for (peer_id, addr) in peers {
                    info!(%peer_id, %addr, "mDNS peer discovered");
                    self.peer_store.record_peer(peer_id, addr.clone());
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr);
                    let _ = self.event_tx.send(NodeEvent::PeerDiscovered(peer_id)).await;
                }
            }
            NousBehaviourEvent::Mdns(mdns::Event::Expired(peers)) => {
                for (peer_id, _) in peers {
                    debug!(%peer_id, "mDNS peer expired");
                }
            }
            NousBehaviourEvent::Kademlia(kad::Event::OutboundQueryProgressed {
                result, ..
            }) => match result {
                kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(record))) => {
                    let key_str =
                        String::from_utf8_lossy(record.record.key.as_ref()).to_string();
                    debug!(key = %key_str, "DHT record found");
                    let _ = self
                        .event_tx
                        .send(NodeEvent::DhtRecordFound {
                            key: key_str,
                            value: record.record.value,
                        })
                        .await;
                }
                kad::QueryResult::PutRecord(Ok(kad::PutRecordOk { key })) => {
                    let key_str = String::from_utf8_lossy(key.as_ref()).to_string();
                    debug!(key = %key_str, "DHT record stored");
                    let _ = self
                        .event_tx
                        .send(NodeEvent::DhtRecordStored { key: key_str })
                        .await;
                }
                kad::QueryResult::GetRecord(Err(e)) => {
                    debug!(?e, "DHT get record failed");
                }
                kad::QueryResult::PutRecord(Err(e)) => {
                    debug!(?e, "DHT put record failed");
                }
                _ => {}
            },
            NousBehaviourEvent::Identify(identify::Event::Received { peer_id, info, .. }) => {
                debug!(%peer_id, protocol = ?info.protocol_version, "identify received");
                for addr in info.listen_addrs {
                    self.peer_store.record_peer(peer_id, addr.clone());
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = NodeConfig::default();
        assert!(config.enable_mdns);
        assert!(config.enable_relay);
        assert_eq!(config.max_connections, 128);
        assert_eq!(config.max_known_peers, 1024);
        assert!(!config.require_signatures);
        assert_eq!(config.rate_limit_burst, 60);
        assert_eq!(config.rate_limit_per_second, 10);
    }

    #[test]
    fn config_serializes() {
        let config = NodeConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: NodeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.max_connections, config.max_connections);
        assert_eq!(deserialized.max_known_peers, config.max_known_peers);
        assert_eq!(deserialized.require_signatures, config.require_signatures);
    }

    #[test]
    fn config_with_bootstrap_peers() {
        let config = NodeConfig {
            bootstrap_peers: vec![
                "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ"
                    .to_string(),
            ],
            ..Default::default()
        };
        assert_eq!(config.bootstrap_peers.len(), 1);
    }

    #[tokio::test]
    async fn node_creates_successfully() {
        let config = NodeConfig::default();
        let (node, _rx) = NousNode::new(&config).expect("node should create");
        assert!(node.connected_peers().is_empty());
        assert!(node.subscribed_topics().is_empty());
        assert!(node.peer_store().is_empty());
        assert_eq!(node.connection_manager().connected_count(), 0);
    }

    #[tokio::test]
    async fn node_has_unique_peer_id() {
        let config = NodeConfig::default();
        let (node1, _) = NousNode::new(&config).unwrap();
        let (node2, _) = NousNode::new(&config).unwrap();
        assert_ne!(node1.local_peer_id(), node2.local_peer_id());
    }

    #[tokio::test]
    async fn node_subscribes_to_topics() {
        let config = NodeConfig::default();
        let (mut node, _rx) = NousNode::new(&config).unwrap();
        node.subscribe(&NousTopic::Messages).unwrap();
        assert!(node.subscribed_topics().contains(&NousTopic::Messages));
    }

    #[tokio::test]
    async fn node_subscribes_all_default() {
        let config = NodeConfig::default();
        let (mut node, _rx) = NousNode::new(&config).unwrap();
        node.subscribe_all_default().unwrap();
        assert_eq!(node.subscribed_topics().len(), 7);
    }

    #[tokio::test]
    async fn node_listens_on_address() {
        let config = NodeConfig::default();
        let (mut node, _rx) = NousNode::new(&config).unwrap();
        assert!(node.listen_on("/ip4/127.0.0.1/tcp/0").is_ok());
    }

    #[tokio::test]
    async fn node_rejects_invalid_address() {
        let config = NodeConfig::default();
        let (mut node, _rx) = NousNode::new(&config).unwrap();
        assert!(node.listen_on("not-a-valid-multiaddr").is_err());
    }

    #[tokio::test]
    async fn publish_rejects_unsigned_when_required() {
        let config = NodeConfig {
            require_signatures: true,
            ..Default::default()
        };
        let (mut node, _rx) = NousNode::new(&config).unwrap();
        node.subscribe(&NousTopic::Messages).unwrap();

        let msg = WireMessage::new(
            NousTopic::Messages,
            b"unsigned".to_vec(),
            "did:key:z123".to_string(),
        );

        assert!(node.publish(&msg).is_err());
    }

    #[tokio::test]
    async fn publish_allows_unsigned_when_not_required() {
        let config = NodeConfig {
            require_signatures: false,
            ..Default::default()
        };
        let (mut node, _rx) = NousNode::new(&config).unwrap();
        node.subscribe(&NousTopic::Messages).unwrap();
        node.listen_on("/ip4/127.0.0.1/tcp/0").unwrap();

        let msg = WireMessage::new(
            NousTopic::Messages,
            b"unsigned-ok".to_vec(),
            "did:key:z123".to_string(),
        );

        // This may fail due to "InsufficientPeers" but should NOT fail due to
        // signature check.
        let result = node.publish(&msg);
        if let Err(e) = &result {
            let err_str = e.to_string();
            assert!(
                err_str.contains("InsufficientPeers") || err_str.contains("insufficient peers"),
                "unexpected error: {err_str}"
            );
        }
    }

    #[tokio::test]
    async fn node_add_peer_records_in_store() {
        let config = NodeConfig::default();
        let (mut node, _rx) = NousNode::new(&config).unwrap();

        let peer = PeerId::random();
        let addr: Multiaddr = "/ip4/192.168.1.1/tcp/4001".parse().unwrap();
        node.add_peer(peer, addr);

        assert!(node.peer_store().get_peer(&peer).is_some());
    }

    #[tokio::test]
    async fn node_export_import_peers() {
        let config = NodeConfig::default();
        let (mut node, _rx) = NousNode::new(&config).unwrap();

        let peer = PeerId::random();
        let addr: Multiaddr = "/ip4/10.0.0.1/tcp/4001".parse().unwrap();
        node.add_peer(peer, addr);

        let exported = node.export_peers();
        assert_eq!(exported.len(), 1);

        let (mut node2, _rx2) = NousNode::new(&config).unwrap();
        node2.import_peers(exported);
        assert_eq!(node2.peer_store().len(), 1);
    }

    #[tokio::test]
    async fn two_nodes_discover_each_other() {
        let config = NodeConfig::default();
        let (mut node1, mut rx1) = NousNode::new(&config).unwrap();
        let (mut node2, mut rx2) = NousNode::new(&config).unwrap();

        node1.listen_on("/ip4/127.0.0.1/tcp/0").unwrap();
        node2.listen_on("/ip4/127.0.0.1/tcp/0").unwrap();

        node1.subscribe_all_default().unwrap();
        node2.subscribe_all_default().unwrap();

        let peer1 = node1.local_peer_id();
        let peer2 = node2.local_peer_id();

        let handle1 = tokio::spawn(async move { node1.run().await });
        let handle2 = tokio::spawn(async move { node2.run().await });

        let discovered = tokio::time::timeout(Duration::from_secs(15), async {
            let mut found_peer1 = false;
            let mut found_peer2 = false;

            loop {
                tokio::select! {
                    Some(event) = rx1.recv() => {
                        if let NodeEvent::PeerDiscovered(p) | NodeEvent::PeerConnected(p) = event {
                            if p == peer2 { found_peer2 = true; }
                        }
                    }
                    Some(event) = rx2.recv() => {
                        if let NodeEvent::PeerDiscovered(p) | NodeEvent::PeerConnected(p) = event {
                            if p == peer1 { found_peer1 = true; }
                        }
                    }
                }

                if found_peer1 && found_peer2 {
                    return true;
                }
            }
        })
        .await;

        handle1.abort();
        handle2.abort();

        assert!(
            discovered.is_ok(),
            "peers should discover each other via mDNS"
        );
        assert!(discovered.unwrap());
    }
}
