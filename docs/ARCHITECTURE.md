# Nous Architecture

## Overview

Nous is a decentralized everything-app: self-sovereign identity, encrypted messaging, on-chain governance, P2P payments, AI inference, and a marketplace — unified under one protocol.

**Design Philosophy: Infinite Minimalism.** Every feature earns its place. No bloat, no abstractions that don't pay for themselves. The system is local-first, privacy-preserving, and cryptographically verifiable. Each crate does one thing well, composes cleanly, and ships fast. Correctness first, then performance — but it should be fast.

Built on libp2p, ed25519, AES-256-GCM, and CRDTs. Runs everywhere: web, mobile, desktop, terminal, CLI.

---

## Crate Map

| Crate | Description |
|---|---|
| `nous-core` | Core types, error handling, shared traits, and configuration |
| `nous-crypto` | ed25519 signing, x25519 key exchange, AES-256-GCM encryption, HKDF derivation, Schnorr ZK proofs |
| `nous-identity` | DID:key generation, verifiable credentials, and reputation scoring |
| `nous-net` | libp2p networking — gossipsub, Kademlia DHT, relay, NAT traversal, mDNS |
| `nous-messaging` | Channels, messages, X3DH key agreement, Double Ratchet protocol |
| `nous-social` | Events, feeds, follow graph, profiles, reactions |
| `nous-governance` | DAOs, proposals, quadratic voting, delegation, vote tallies |
| `nous-payments` | Wallets, transfers, escrow, invoices |
| `nous-storage` | SQLite-backed persistent key-value store with CRDT support |
| `nous-ai` | Local AI inference, agent framework, semantic search |
| `nous-marketplace` | P2P listings, purchases, reviews, seller ratings |
| `nous-browser` | Decentralized web browser — IPFS gateway, ENS resolution, dApp browser |
| `nous-files` | Versioned file storage with content-addressed deduplication |
| `nous-nostr` | NIP-01/02/04/09 relay implementation |
| `nous-api` | REST + GraphQL + gRPC server built on Axum |
| `nous-tui` | Terminal interface built on ratatui |
| `nous-cli` | Command-line interface built on clap |
| `nous-wasm` | Browser-side cryptographic operations via WebAssembly |
| `nous-integration` | End-to-end integration test suite |
| `nous-bench` | Performance benchmarks (Criterion) |

**Total: 20 crates** in the workspace, plus platform apps (web, android, ios, desktop).

---

## Platform Targets

| Platform | Stack | Notes |
|---|---|---|
| Web | Next.js PWA | Offline-capable, `nous-wasm` for crypto |
| Android | Kotlin + Jetpack Compose | Native UI, Rust core via JNI |
| iOS | Swift + SwiftUI | Native UI, Rust core via UniFFI |
| Desktop | Tauri | Linux/macOS, Rust backend + web frontend |
| Terminal | ratatui (`nous-tui`) | Full-featured TUI for power users |
| CLI | clap (`nous-cli`) | Scriptable, JSON output |

---

## Dependency Graph

```
                      nous-core
                     /    |    \
              nous-crypto  nous-storage
                 |
            nous-identity
            /    |    \
     nous-social nous-messaging nous-governance
           \     |      /        |
       nous-marketplace   nous-payments
                |              |
            nous-files     nous-nostr
                |
            nous-ai
                \
            nous-net
                |
      ┌─────────┼─────────┐
      │         │         │
  nous-api  nous-tui  nous-cli
      │
  nous-wasm
```

`nous-core` sits at the root. `nous-crypto` and `nous-storage` provide the cryptographic and persistence foundations. Domain crates (`social`, `messaging`, `governance`, `marketplace`, `payments`) build on identity. Surface crates (`api`, `tui`, `cli`) compose everything into user-facing interfaces. `nous-wasm` exposes crypto to the browser.

---

## API Architecture

| Protocol | Endpoint | Purpose |
|---|---|---|
| REST | `:8080` | Standard HTTP API, OpenAPI/Swagger docs via `utoipa` |
| GraphQL | `:8080/graphql` | Flexible queries for frontend clients (`async-graphql`) |
| gRPC | `:8081` | High-performance service-to-service calls (`tonic` + `prost`) |
| Nostr | `:9735` | NIP-compliant relay for Nostr client interop |

The API server is built on **Axum** with tower middleware for auth, rate limiting, and tracing. All endpoints require ed25519 signature authentication.

---

## Security Model

| Layer | Mechanism |
|---|---|
| Identity | ed25519 key pairs, DID:key identifiers |
| Signing | ed25519 signatures on all mutations |
| Key Exchange | X3DH (Extended Triple Diffie-Hellman) for session setup |
| Message Encryption | Double Ratchet protocol (forward secrecy + break-in recovery) |
| Symmetric Encryption | AES-256-GCM for data at rest and in transit |
| Key Derivation | HKDF-SHA256 for deriving symmetric keys from shared secrets |
| Zero-Knowledge | Schnorr ZK proofs for credential verification without disclosure |
| Access Control | Capability-based — tokens grant specific permissions, no ambient authority |
| Secret Management | `zeroize` for memory cleanup, argon2 for password hashing |

All cryptographic operations use audited Rust crates (`ed25519-dalek`, `x25519-dalek`, `aes-gcm`, `hkdf`). No custom crypto.

---

## Data Flow

```
User Action
    │
    ▼
Local SQLite (source of truth)
    │
    ├── CRDT merge ──► Conflict resolution
    │
    ▼
libp2p gossipsub (peer broadcast)
    │
    ├── Kademlia DHT ──► Peer discovery + content routing
    │
    └── IPFS ──► Content-addressed distributed storage
```

**Local-first**: All data is written to SQLite before network propagation. The app works fully offline. Sync happens opportunistically when peers are available.

**CRDTs**: Conflict-free Replicated Data Types resolve concurrent edits without coordination. No central server decides merge order.

**Content addressing**: Files and large objects are stored via content-addressed hashing (IPFS). Deduplication is automatic. Integrity is verifiable.

**Encryption**: All data leaving the device is encrypted. Messages use Double Ratchet. Files use AES-256-GCM. Only the owner's keys can decrypt.
