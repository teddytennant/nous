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
use crate::events::{NodeEvent, WireMessage};
use crate::topics::NousTopic;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub listen_addresses: Vec<String>,
    pub bootstrap_peers: Vec<String>,
    pub enable_mdns: bool,
    pub enable_relay: bool,
    pub max_connections: usize,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            listen_addresses: vec!["/ip4/0.0.0.0/tcp/0".to_string()],
            bootstrap_peers: Vec::new(),
            enable_mdns: true,
            enable_relay: true,
            max_connections: 128,
        }
    }
}

pub struct NousNode {
    swarm: Swarm<NousBehaviour>,
    event_tx: mpsc::Sender<NodeEvent>,
    connected_peers: HashSet<PeerId>,
    subscribed_topics: HashSet<NousTopic>,
    local_peer_id: PeerId,
}

impl NousNode {
    pub fn new(
        _config: &NodeConfig,
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

        Ok((
            Self {
                swarm,
                event_tx,
                connected_peers: HashSet::new(),
                subscribed_topics: HashSet::new(),
                local_peer_id,
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
        self.swarm
            .behaviour_mut()
            .kademlia
            .add_address(&peer_id, addr);
    }

    pub async fn run(&mut self) {
        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::Behaviour(event) => self.handle_behaviour_event(event).await,
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!(%address, "listening on");
                    let _ = self.event_tx.send(NodeEvent::ListeningOn(address)).await;
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    self.connected_peers.insert(peer_id);
                    info!(%peer_id, peers = self.connected_peers.len(), "peer connected");
                    let _ = self.event_tx.send(NodeEvent::PeerConnected(peer_id)).await;
                }
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    self.connected_peers.remove(&peer_id);
                    info!(%peer_id, peers = self.connected_peers.len(), "peer disconnected");
                    let _ = self
                        .event_tx
                        .send(NodeEvent::PeerDisconnected(peer_id))
                        .await;
                }
                SwarmEvent::OutgoingConnectionError { error, .. } => {
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
                let topic = NousTopic::from_topic_hash(&message.topic)
                    .unwrap_or(NousTopic::Custom(message.topic.to_string()));

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
                    debug!(
                        key = ?String::from_utf8_lossy(record.record.key.as_ref()),
                        "DHT record found"
                    );
                }
                kad::QueryResult::PutRecord(Ok(kad::PutRecordOk { key })) => {
                    debug!(key = ?String::from_utf8_lossy(key.as_ref()), "DHT record stored");
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
    }

    #[test]
    fn config_serializes() {
        let config = NodeConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let _: NodeConfig = serde_json::from_str(&json).unwrap();
    }

    #[tokio::test]
    async fn node_creates_successfully() {
        let config = NodeConfig::default();
        let (node, _rx) = NousNode::new(&config).expect("node should create");
        assert!(node.connected_peers().is_empty());
        assert!(node.subscribed_topics().is_empty());
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

        // Wait for mDNS discovery (with timeout)
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
