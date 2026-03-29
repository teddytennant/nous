# Nous

Decentralized everything-app. Self-sovereign identity, encrypted messaging, on-chain governance, P2P payments, AI inference — unified under one protocol.

Built on libp2p, ed25519, AES-256-GCM, zk-SNARKs. Local-first with CRDTs. Runs everywhere: web, mobile, desktop, terminal, CLI.

## Architecture

```
nous-core        Core types, traits, configuration
nous-crypto      Cryptographic primitives (ed25519, x25519, AES-256-GCM, key derivation)
nous-identity    DID:key identifiers, verifiable credentials, reputation
nous-net         P2P networking via libp2p (gossip, DHT, relay, NAT traversal)
nous-messaging   End-to-end encrypted messaging (1:1, group, channels)
nous-social      Decentralized social feeds, posts, follows, reactions
nous-governance  On-chain voting, quadratic voting, delegation, proposals, DAOs
nous-payments    Multi-chain wallet, send/receive/swap, escrow, invoices
nous-storage     Local-first storage (SQLite + CRDTs), IPFS, encrypted vaults
nous-ai          Local inference, agent framework, semantic search
nous-marketplace P2P marketplace for goods, services, digital assets
nous-browser     Decentralized web browser (IPFS gateway, ENS, dApp browser)
nous-api         API server (REST + GraphQL + gRPC)
nous-tui         Terminal interface (ratatui)
nous-cli         Command-line interface
```

## Platforms

- **Web** — Next.js PWA, offline-capable
- **Android** — Kotlin + Jetpack Compose
- **iOS** — Swift + SwiftUI
- **Linux/macOS** — Tauri desktop app
- **Terminal** — Full-featured TUI
- **CLI** — Scriptable, JSON output

## Build

```bash
cargo build --workspace
cargo test --workspace
```

## License

MIT
