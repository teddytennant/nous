//! `pouw_cli`: tiny REST client for a running `nous-pouw-node`.
//!
//! Usage:
//! ```text
//!   pouw_cli status --rpc http://127.0.0.1:8901
//!   pouw_cli balance --did <did:key:z...> --rpc http://127.0.0.1:8901
//!   pouw_cli send-tx --from-idx 0 --to-idx 1 --amount 100 --nonce 1 --rpc http://127.0.0.1:8901
//!   pouw_cli stake --from-idx 0 --amount 50 --nonce 1 --rpc http://127.0.0.1:8901
//! ```
//!
//! `--from-idx` / `--to-idx` are validator indexes from the deterministic
//! genesis (seed=0). For real key management, replace this with a flow that
//! loads SigningKeys from disk.

use std::env;

use ed25519_dalek::SigningKey;
use nous_pouw::state::WorkerId;
use nous_pouw::tx::{Transaction, TxBody};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

fn arg<T: std::str::FromStr>(args: &[String], flag: &str, default: T) -> T {
    for w in args.windows(2) {
        if w[0] == flag
            && let Ok(v) = w[1].parse()
        {
            return v;
        }
    }
    default
}

fn arg_str(args: &[String], flag: &str) -> Option<String> {
    for w in args.windows(2) {
        if w[0] == flag {
            return Some(w[1].clone());
        }
    }
    None
}

fn validator_keys(n: usize) -> Vec<SigningKey> {
    let mut rng = ChaCha20Rng::seed_from_u64(0);
    (0..n).map(|_| SigningKey::generate(&mut rng)).collect()
}

fn idx_to_id_and_sk(idx: usize, n: usize) -> (SigningKey, WorkerId) {
    let sks = validator_keys(n);
    let sk = SigningKey::from_bytes(&sks[idx].to_bytes());
    let id = WorkerId::from_verifying_key(&sk.verifying_key());
    (sk, id)
}

fn id_to_did(id: &WorkerId) -> String {
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&id.0).expect("valid id bytes");
    nous_crypto::keys::public_key_to_did(&vk)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
    let rpc = arg_str(&args, "--rpc").unwrap_or_else(|| "http://127.0.0.1:8901".into());
    let n_validators: usize = arg(&args, "--validators", 4usize);

    match cmd {
        "status" => {
            let body = http_get(&format!("{rpc}/status")).await?;
            println!("{body}");
        }
        "balance" => {
            let did = match arg_str(&args, "--did") {
                Some(d) => d,
                None => {
                    let idx: usize = arg(&args, "--idx", 0);
                    let (_, id) = idx_to_id_and_sk(idx, n_validators);
                    id_to_did(&id)
                }
            };
            let body = http_get(&format!("{rpc}/balance/{did}")).await?;
            println!("{body}");
        }
        "send-tx" => {
            let from_idx: usize = arg(&args, "--from-idx", 0);
            let to_idx: usize = arg(&args, "--to-idx", 1);
            let amount: u64 = arg(&args, "--amount", 100);
            let nonce: u64 = arg(&args, "--nonce", 1);
            let (sk, from_id) = idx_to_id_and_sk(from_idx, n_validators);
            let (_, to_id) = idx_to_id_and_sk(to_idx, n_validators);
            let tx = Transaction::new_signed(
                TxBody::Transfer {
                    from: from_id,
                    to: to_id,
                    amount,
                },
                nonce,
                0,
                &sk,
            );
            let body = serde_json::json!({ "tx": tx }).to_string();
            let resp = http_post_json(&format!("{rpc}/tx"), &body).await?;
            println!("{resp}");
        }
        "stake" => {
            let from_idx: usize = arg(&args, "--from-idx", 0);
            let amount: u64 = arg(&args, "--amount", 100);
            let nonce: u64 = arg(&args, "--nonce", 1);
            let (sk, from_id) = idx_to_id_and_sk(from_idx, n_validators);
            let tx = Transaction::new_signed(
                TxBody::Stake {
                    worker: from_id,
                    amount,
                },
                nonce,
                0,
                &sk,
            );
            let body = serde_json::json!({ "tx": tx }).to_string();
            let resp = http_post_json(&format!("{rpc}/tx"), &body).await?;
            println!("{resp}");
        }
        "register-validator" => {
            let from_idx: usize = arg(&args, "--from-idx", 0);
            let nonce: u64 = arg(&args, "--nonce", 1);
            let (sk, from_id) = idx_to_id_and_sk(from_idx, n_validators);
            let tx = Transaction::new_signed(
                TxBody::RegisterValidator { worker: from_id },
                nonce,
                0,
                &sk,
            );
            let body = serde_json::json!({ "tx": tx }).to_string();
            let resp = http_post_json(&format!("{rpc}/tx"), &body).await?;
            println!("{resp}");
        }
        "did" => {
            let idx: usize = arg(&args, "--idx", 0);
            let (_, id) = idx_to_id_and_sk(idx, n_validators);
            println!("{}", id_to_did(&id));
        }
        _ => {
            eprintln!("usage: pouw_cli <status|balance|send-tx|stake|register-validator|did> [...]");
            std::process::exit(1);
        }
    }
    Ok(())
}

async fn http_get(url: &str) -> std::io::Result<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    let stripped = url.strip_prefix("http://").unwrap_or(url);
    let (host, path) = match stripped.split_once('/') {
        Some((h, p)) => (h, format!("/{p}")),
        None => (stripped, "/".into()),
    };
    let mut sock = TcpStream::connect(host).await?;
    let req = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    sock.write_all(req.as_bytes()).await?;
    let mut buf = Vec::new();
    sock.read_to_end(&mut buf).await?;
    let s = String::from_utf8_lossy(&buf).to_string();
    Ok(s.split_once("\r\n\r\n").map(|x| x.1.to_string()).unwrap_or(s))
}

async fn http_post_json(url: &str, body: &str) -> std::io::Result<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    let stripped = url.strip_prefix("http://").unwrap_or(url);
    let (host, path) = match stripped.split_once('/') {
        Some((h, p)) => (h, format!("/{p}")),
        None => (stripped, "/".into()),
    };
    let mut sock = TcpStream::connect(host).await?;
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    sock.write_all(req.as_bytes()).await?;
    let mut buf = Vec::new();
    sock.read_to_end(&mut buf).await?;
    let s = String::from_utf8_lossy(&buf).to_string();
    Ok(s.split_once("\r\n\r\n").map(|x| x.1.to_string()).unwrap_or(s))
}
