//! Real multi-OS-process end-to-end test.
//!
//! Builds the `node` example binary, spawns 4 processes on loopback ports,
//! waits for them to mesh + finalize blocks via real libp2p gossipsub, and
//! asserts that all 4 nodes converge on the same `head_hash`.
//!
//! Skipped in `cargo test` if `NOUS_POUW_E2E` is not set, because cold-build
//! plus 4-process spawn is heavyweight and CI may not always have the binary
//! pre-built. Run locally with:
//!
//! ```text
//! NOUS_POUW_E2E=1 cargo test -p nous-pouw --test multi_process -- --nocapture
//! ```

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn four_real_processes_converge_via_libp2p() {
    if std::env::var("NOUS_POUW_E2E").is_err() {
        eprintln!("skipping: NOUS_POUW_E2E not set");
        return;
    }

    // Build the binary once before spawning.
    let status = Command::new("cargo")
        .args(["build", "--release", "-p", "nous-pouw", "--example", "node"])
        .status()
        .expect("cargo build");
    assert!(status.success(), "cargo build failed");

    let target = workspace_root().join("target/release/examples/node");
    assert!(target.exists(), "binary not at expected path");

    let tmp = tempfile::tempdir().expect("tempdir");

    // Allocate 8 ports — first set is libp2p, second set is RPC.
    let p2p_base = 19501u16;
    let rpc_base = 19601u16;
    let n = 4usize;

    let mut children: Vec<Child> = Vec::new();

    // Bootstrap node.
    let db0 = tmp.path().join("pouw-0.db");
    children.push(
        Command::new(&target)
            .args([
                "--listen",
                &format!("/ip4/127.0.0.1/tcp/{}", p2p_base),
                "--rpc",
                &format!("127.0.0.1:{}", rpc_base),
                "--idx",
                "0",
                "--validators",
                &n.to_string(),
                "--genesis-balance",
                "10000",
                "--db",
                db0.to_str().unwrap(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn node 0"),
    );
    std::thread::sleep(Duration::from_millis(2_000));

    // Followers.
    for i in 1..n {
        let db = tmp.path().join(format!("pouw-{i}.db"));
        children.push(
            Command::new(&target)
                .args([
                    "--listen",
                    &format!("/ip4/127.0.0.1/tcp/{}", p2p_base + i as u16),
                    "--rpc",
                    &format!("127.0.0.1:{}", rpc_base + i as u16),
                    "--bootstrap",
                    &format!("/ip4/127.0.0.1/tcp/{}", p2p_base),
                    "--idx",
                    &i.to_string(),
                    "--validators",
                    &n.to_string(),
                    "--genesis-balance",
                    "10000",
                    "--db",
                    db.to_str().unwrap(),
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn follower"),
        );
    }

    // Wait up to 60s for height >= 2 on every node and matching head_hash.
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut heights = vec![0u64; n];
    let mut hashes = vec![String::new(); n];
    loop {
        for i in 0..n {
            let url = format!("http://127.0.0.1:{}/status", rpc_base + i as u16);
            if let Some((h, hash)) = poll_status(&url) {
                heights[i] = h;
                hashes[i] = hash;
            }
        }
        let min_h = *heights.iter().min().unwrap_or(&0);
        let all_same = hashes.windows(2).all(|w| w[0] == w[1] && !w[0].is_empty());
        if min_h >= 2 && all_same {
            break;
        }
        if Instant::now() > deadline {
            for c in &mut children {
                let _ = c.kill();
            }
            panic!("did not converge within 60s; heights={heights:?} hashes={hashes:?}");
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    // Tear down children.
    for c in &mut children {
        let _ = c.kill();
    }
    for c in &mut children {
        let _ = c.wait();
    }
}

fn workspace_root() -> PathBuf {
    // tests/multi_process.rs lives at <root>/crates/nous-pouw/tests/, so go up 3.
    let cargo_manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    cargo_manifest
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

fn poll_status(url: &str) -> Option<(u64, String)> {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    let stripped = url.strip_prefix("http://")?;
    let (host, path) = stripped
        .split_once('/')
        .map(|(h, p)| (h, format!("/{p}")))?;
    let mut sock =
        TcpStream::connect_timeout(&host.parse().ok()?, Duration::from_millis(500)).ok()?;
    sock.set_read_timeout(Some(Duration::from_millis(2_000)))
        .ok()?;
    let req = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    sock.write_all(req.as_bytes()).ok()?;
    let mut buf = Vec::new();
    sock.read_to_end(&mut buf).ok()?;
    let s = String::from_utf8_lossy(&buf).to_string();
    let body = s.split_once("\r\n\r\n").map(|x| x.1)?;
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    Some((
        v["height"].as_u64().unwrap_or(0),
        v["head_hash_hex"].as_str().unwrap_or("").to_string(),
    ))
}
