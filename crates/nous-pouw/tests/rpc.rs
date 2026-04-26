//! RPC smoke test: spawn a single PouwNode + axum server, hit the endpoints.

use std::sync::Arc;
use std::time::Duration;

use ed25519_dalek::SigningKey;
use nous_pouw::engine::{Engine, EngineConfig};
use nous_pouw::node::{NodeConfig, PouwNode};
use nous_pouw::rpc::{NodeHandle, NodeSnapshot, RpcServer};
use nous_pouw::sim::ConfigurableExecutor;
use nous_pouw::state::{ChainState, WorkerId};
use nous_pouw::tx::{Transaction, TxBody};
use nous_pouw::Network;
use rand::rngs::OsRng;

/// Trivial in-process Network impl for the RPC test — we don't need real gossip.
struct NoopNet;
impl Network for NoopNet {
    fn publish(&self, _topic: nous_pouw::Topic, _from: WorkerId, _payload: Vec<u8>) {}
    fn drain(&self) -> Vec<nous_pouw::NetworkEvent> {
        Vec::new()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rpc_status_endpoint_returns_json() {
    let sk = SigningKey::generate(&mut OsRng);
    let mut state = ChainState::new();
    let id = WorkerId::from_verifying_key(&sk.verifying_key());
    state.register_worker(id, 1_000, 1.0);
    state.validators.insert(id);
    state.workers.get_mut(&id).unwrap().balance = 500;

    let engine = Engine::new(state, EngineConfig::default());
    let executor = ConfigurableExecutor::new(&[SigningKey::from_bytes(&sk.to_bytes())]);
    let node = Arc::new(PouwNode::spawn(
        SigningKey::from_bytes(&sk.to_bytes()),
        engine,
        executor,
        Arc::new(NoopNet),
        None,
        NodeConfig::default(),
    ));

    let rpc = RpcServer::spawn(
        node.clone() as Arc<dyn NodeHandle>,
        "127.0.0.1:0".parse().unwrap(),
    )
    .await
    .unwrap();
    let url = format!("http://{}", rpc.local_addr);

    // Tiny pure-stdlib HTTP client via tokio TcpStream + manual request.
    // We avoid pulling reqwest into nous-pouw deps just for tests.
    let body = http_get(&format!("{url}/status")).await;
    let snap: NodeSnapshot = serde_json::from_str(&body).expect("status json");
    assert_eq!(snap.validators, 1);
    assert_eq!(snap.workers, 1);

    let did = nous_crypto::keys::public_key_to_did(&sk.verifying_key());
    let body = http_get(&format!("{url}/balance/{did}")).await;
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["balance"].as_u64(), Some(500));
    assert_eq!(v["stake"].as_u64(), Some(1_000));

    drop(rpc);
    drop(node);
    tokio::time::sleep(Duration::from_millis(50)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rpc_post_tx_validates_and_accepts() {
    let sk = SigningKey::generate(&mut OsRng);
    let mut state = ChainState::new();
    let id = WorkerId::from_verifying_key(&sk.verifying_key());
    state.register_worker(id, 1_000, 1.0);
    state.validators.insert(id);
    state.workers.get_mut(&id).unwrap().balance = 1_000;

    let recipient_sk = SigningKey::generate(&mut OsRng);
    let recipient_id = WorkerId::from_verifying_key(&recipient_sk.verifying_key());
    state.register_worker(recipient_id, 0, 1.0);

    let engine = Engine::new(state, EngineConfig::default());
    let executor = ConfigurableExecutor::new(&[SigningKey::from_bytes(&sk.to_bytes())]);
    let node = Arc::new(PouwNode::spawn(
        SigningKey::from_bytes(&sk.to_bytes()),
        engine,
        executor,
        Arc::new(NoopNet),
        None,
        NodeConfig::default(),
    ));

    let rpc = RpcServer::spawn(
        node.clone() as Arc<dyn NodeHandle>,
        "127.0.0.1:0".parse().unwrap(),
    )
    .await
    .unwrap();
    let url = format!("http://{}", rpc.local_addr);

    let tx = Transaction::new_signed(
        TxBody::Transfer {
            from: id,
            to: recipient_id,
            amount: 50,
        },
        1,
        0,
        &SigningKey::from_bytes(&sk.to_bytes()),
    );
    let req_body = serde_json::json!({ "tx": tx });
    let resp = http_post_json(&format!("{url}/tx"), &req_body.to_string()).await;
    let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert!(v["tx_id_hex"].is_string());
    assert_eq!(node.mempool_len(), 1);

    drop(rpc);
    drop(node);
    tokio::time::sleep(Duration::from_millis(50)).await;
}

// Pure-tokio HTTP/1.1 GET — small enough to inline so the crate doesn't take a reqwest dep.
async fn http_get(url: &str) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    let stripped = url.strip_prefix("http://").unwrap();
    let (host, path) = match stripped.split_once('/') {
        Some((h, p)) => (h, format!("/{p}")),
        None => (stripped, "/".into()),
    };
    let mut sock = TcpStream::connect(host).await.unwrap();
    let req = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    sock.write_all(req.as_bytes()).await.unwrap();
    let mut buf = Vec::new();
    sock.read_to_end(&mut buf).await.unwrap();
    let s = String::from_utf8_lossy(&buf).to_string();
    s.split_once("\r\n\r\n").map(|x| x.1.to_string()).unwrap_or(s)
}

async fn http_post_json(url: &str, body: &str) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    let stripped = url.strip_prefix("http://").unwrap();
    let (host, path) = match stripped.split_once('/') {
        Some((h, p)) => (h, format!("/{p}")),
        None => (stripped, "/".into()),
    };
    let mut sock = TcpStream::connect(host).await.unwrap();
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    sock.write_all(req.as_bytes()).await.unwrap();
    let mut buf = Vec::new();
    sock.read_to_end(&mut buf).await.unwrap();
    let s = String::from_utf8_lossy(&buf).to_string();
    s.split_once("\r\n\r\n").map(|x| x.1.to_string()).unwrap_or(s)
}
