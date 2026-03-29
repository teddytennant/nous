use libp2p::gossipsub::{self, MessageAuthenticity, ValidationMode};
use libp2p::identify;
use libp2p::kad;
use libp2p::mdns;
use libp2p::relay;
use libp2p::{PeerId, dcutr};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use crate::topics::NousTopic;

#[derive(libp2p::swarm::NetworkBehaviour)]
pub struct NousBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    pub mdns: mdns::tokio::Behaviour,
    pub identify: identify::Behaviour,
    pub relay_client: relay::client::Behaviour,
    pub dcutr: dcutr::Behaviour,
}

impl NousBehaviour {
    pub fn new(
        local_peer_id: PeerId,
        local_key: &libp2p::identity::Keypair,
        relay_behaviour: relay::client::Behaviour,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(ValidationMode::Strict)
            .message_id_fn(|message| {
                let mut hasher = DefaultHasher::new();
                message.data.hash(&mut hasher);
                message.source.hash(&mut hasher);
                gossipsub::MessageId::from(hasher.finish().to_be_bytes().to_vec())
            })
            .max_transmit_size(1024 * 1024) // 1 MiB
            .build()
            .map_err(|e| format!("gossipsub config error: {e}"))?;

        let gossipsub = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )
        .map_err(|e| format!("gossipsub behaviour error: {e}"))?;

        let kademlia = {
            let store = kad::store::MemoryStore::new(local_peer_id);
            let mut config = kad::Config::new(
                libp2p::StreamProtocol::try_from_owned("/nous/kad/1.0.0".to_string())
                    .expect("valid protocol"),
            );
            config.set_query_timeout(Duration::from_secs(60));
            config.set_replication_factor(std::num::NonZeroUsize::new(3).expect("nonzero"));
            kad::Behaviour::with_config(local_peer_id, store, config)
        };

        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?;

        let identify = identify::Behaviour::new(identify::Config::new(
            "/nous/id/1.0.0".to_string(),
            local_key.public(),
        ));

        let dcutr = dcutr::Behaviour::new(local_peer_id);

        Ok(Self {
            gossipsub,
            kademlia,
            mdns,
            identify,
            relay_client: relay_behaviour,
            dcutr,
        })
    }

    pub fn subscribe(&mut self, topic: &NousTopic) -> Result<bool, gossipsub::SubscriptionError> {
        self.gossipsub.subscribe(&topic.as_topic())
    }

    pub fn publish(
        &mut self,
        topic: &NousTopic,
        data: impl Into<Vec<u8>>,
    ) -> Result<gossipsub::MessageId, gossipsub::PublishError> {
        self.gossipsub.publish(topic.as_topic(), data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gossipsub_message_id_deterministic() {
        let mut hasher1 = DefaultHasher::new();
        b"test data".hash(&mut hasher1);
        let id1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        b"test data".hash(&mut hasher2);
        let id2 = hasher2.finish();

        assert_eq!(id1, id2);
    }
}
