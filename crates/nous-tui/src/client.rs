use serde::Deserialize;

#[derive(Clone)]
pub struct ApiClient {
    base_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct FeedResponse {
    pub events: Vec<FeedEvent>,
    pub count: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeedEvent {
    pub id: String,
    pub pubkey: String,
    pub created_at: String,
    pub kind: u32,
    pub content: String,
    pub tags: Vec<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ChannelListItem {
    pub id: String,
    pub kind: String,
    pub name: Option<String>,
    pub members: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageItem {
    pub id: String,
    pub channel_id: String,
    pub sender: String,
    pub content: String,
    pub reply_to: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct IdentityResponse {
    pub did: String,
    pub display_name: Option<String>,
    pub signing_key_type: String,
    pub exchange_key_type: String,
}

#[derive(Debug, Deserialize)]
pub struct WalletResponse {
    pub did: String,
    pub balances: Vec<BalanceEntry>,
    pub nonce: u64,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BalanceEntry {
    pub token: String,
    pub amount: String,
}

#[derive(Debug, Deserialize)]
pub struct DaoItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub founder_did: String,
    pub member_count: usize,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct DaoListResponse {
    pub daos: Vec<DaoItem>,
    pub count: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProposalItem {
    pub id: String,
    pub dao_id: String,
    pub title: String,
    pub description: String,
    pub proposer_did: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ProposalListResponse {
    pub proposals: Vec<ProposalItem>,
    pub count: usize,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn health(&self) -> Result<HealthResponse, reqwest::Error> {
        self.client
            .get(format!("{}/health", self.base_url))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn feed(&self, limit: usize) -> Result<FeedResponse, reqwest::Error> {
        self.client
            .get(format!("{}/feed?limit={}", self.base_url, limit))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn channels(&self, did: &str) -> Result<Vec<ChannelListItem>, reqwest::Error> {
        self.client
            .get(format!("{}/channels?did={}", self.base_url, did))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn messages(
        &self,
        channel_id: &str,
        limit: usize,
    ) -> Result<Vec<MessageItem>, reqwest::Error> {
        self.client
            .get(format!(
                "{}/channels/{}/messages?limit={}",
                self.base_url, channel_id, limit
            ))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn identity(&self, did: &str) -> Result<IdentityResponse, reqwest::Error> {
        self.client
            .get(format!("{}/identities/{}", self.base_url, did))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn wallet(&self, did: &str) -> Result<WalletResponse, reqwest::Error> {
        self.client
            .get(format!("{}/wallets/{}", self.base_url, did))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn daos(&self) -> Result<DaoListResponse, reqwest::Error> {
        self.client
            .get(format!("{}/daos", self.base_url))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn proposals(&self) -> Result<ProposalListResponse, reqwest::Error> {
        self.client
            .get(format!("{}/proposals", self.base_url))
            .send()
            .await?
            .json()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_creates() {
        let client = ApiClient::new("http://localhost:8080/api/v1");
        assert_eq!(client.base_url, "http://localhost:8080/api/v1");
    }

    #[test]
    fn client_custom_url() {
        let client = ApiClient::new("http://10.0.0.1:9090/api/v1");
        assert_eq!(client.base_url, "http://10.0.0.1:9090/api/v1");
    }

    #[test]
    fn health_response_deserialize() {
        let json = r#"{"status":"ok","version":"0.1.0","uptime_ms":12345}"#;
        let resp: HealthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, "ok");
        assert_eq!(resp.version, "0.1.0");
        assert_eq!(resp.uptime_ms, 12345);
    }

    #[test]
    fn feed_response_deserialize() {
        let json = r#"{"events":[{"id":"abc","pubkey":"did:key:z123","created_at":"2026-03-29T00:00:00Z","kind":1,"content":"hello","tags":[["t","test"]]}],"count":1}"#;
        let resp: FeedResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.events.len(), 1);
        assert_eq!(resp.events[0].content, "hello");
        assert_eq!(resp.events[0].tags[0], vec!["t", "test"]);
    }

    #[test]
    fn wallet_response_deserialize() {
        let json = r#"{"did":"did:key:z123","balances":[{"token":"ETH","amount":"100"}],"nonce":0,"created_at":"2026-03-29"}"#;
        let resp: WalletResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.balances.len(), 1);
        assert_eq!(resp.balances[0].token, "ETH");
    }

    #[test]
    fn identity_response_deserialize() {
        let json = r#"{"did":"did:key:z123","display_name":"Alice","signing_key_type":"ed25519","exchange_key_type":"x25519"}"#;
        let resp: IdentityResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.did, "did:key:z123");
        assert_eq!(resp.display_name, Some("Alice".to_string()));
    }

    #[test]
    fn dao_list_deserialize() {
        let json = r#"{"daos":[{"id":"dao1","name":"Test","description":"desc","founder_did":"did:key:z123","member_count":3,"created_at":"2026-03-29"}],"count":1}"#;
        let resp: DaoListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.daos.len(), 1);
        assert_eq!(resp.daos[0].name, "Test");
    }

    #[test]
    fn proposal_list_deserialize() {
        let json = r#"{"proposals":[{"id":"p1","dao_id":"d1","title":"Fund","description":"desc","proposer_did":"did:key:z123","status":"Active","created_at":"2026-03-29"}],"count":1}"#;
        let resp: ProposalListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.proposals.len(), 1);
        assert_eq!(resp.proposals[0].title, "Fund");
    }

    #[test]
    fn channel_deserialize() {
        let json = r#"[{"id":"ch1","kind":"direct","name":null,"members":["a","b"],"created_at":"2026-03-29"}]"#;
        let resp: Vec<ChannelListItem> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.len(), 1);
        assert_eq!(resp[0].members.len(), 2);
    }

    #[test]
    fn message_deserialize() {
        let json = r#"[{"id":"m1","channel_id":"ch1","sender":"did:key:z123","content":"hi","reply_to":null,"timestamp":"2026-03-29T12:00:00Z"}]"#;
        let resp: Vec<MessageItem> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.len(), 1);
        assert_eq!(resp[0].content, "hi");
    }
}
