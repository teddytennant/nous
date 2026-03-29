# Nous

Decentralized everything-app built in Rust. Self-sovereign identity (DID:key), end-to-end encrypted messaging (Double Ratchet), on-chain governance (quadratic voting), P2P payments, AI inference, a marketplace, and a Nostr relay — all in one protocol. Local-first with CRDTs and SQLite. Runs as a CLI, API server, TUI, web app, desktop app (Tauri), and has iOS/Android shells.

> **Status:** Early development (v0.1.0). Core crates are functional with tests passing. Mobile apps are UI shells without full Rust integration. The web app connects to the API server but not all features are wired up. Treat this as a working prototype, not production software.

## Architecture

```
 ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌──────────┐
 │   CLI   │  │   TUI   │  │ Web App │  │ Desktop  │
 │ (nous)  │  │(ratatui)│  │(Next.js)│  │ (Tauri)  │
 └────┬────┘  └────┬────┘  └────┬────┘  └────┬─────┘
      │            │            │             │
      │            │       ┌────▼────┐        │
      │            │       │   API   │        │
      │            │       │ Server  │◄───────┘
      │            │       │ :8080   │
      │            │       └────┬────┘
      │            │            │
 ┌────▼────────────▼────────────▼─────────────────┐
 │                  Nous Core                      │
 │  identity · crypto · messaging · social         │
 │  governance · payments · marketplace · ai       │
 │  files · storage (SQLite + CRDTs)               │
 └─────────────────────┬──────────────────────────┘
                       │
              ┌────────▼────────┐
              │   P2P Network   │
              │    (libp2p)     │
              │ gossipsub · DHT │
              └─────────────────┘
```

**REST** on `:8080` · **GraphQL** on `:8080/graphql` · **gRPC** on `:8081` · **Nostr relay** on `:9735`

## Quickstart

```bash
git clone https://github.com/teddytennant/nous.git
cd nous
cargo build --workspace
cargo run --bin nous -- init          # create a local identity
cargo run --bin nous-api              # start the API server on :8080
```

## Prerequisites

- **Rust** (edition 2024 — requires nightly or Rust 1.85+)
- **Protobuf compiler** (`protoc`) — needed for gRPC code generation
- **Node.js 20+** and **npm** — only for the web app
- **SQLite** is bundled via `rusqlite` (no system install needed)

## Building

```bash
# Build everything
cargo build --workspace

# Release build (LTO, stripped)
cargo build --workspace --release

# Build only the API server
cargo build --bin nous-api

# Build only the CLI
cargo build --bin nous

# Web app
cd apps/web && npm install && npm run build
```

## Running

### API Server

```bash
# Starts REST on :8080, GraphQL on :8080/graphql, gRPC on :8081
cargo run --bin nous-api

# With debug logging
RUST_LOG=debug cargo run --bin nous-api
```

Swagger docs available at `http://localhost:8080/api-doc` when the server is running.

### CLI

```bash
cargo run --bin nous -- init                          # initialize identity
cargo run --bin nous -- status                        # node info
cargo run --bin nous -- identity show                 # show your DID
cargo run --bin nous -- social post "hello world"     # post to feed
cargo run --bin nous -- social feed                   # view feed
cargo run --bin nous -- wallet balance                # check balances
cargo run --bin nous -- net peers                     # list connected peers
cargo run --bin nous -- terminal                      # launch embedded TUI
cargo run --bin nous -- --json status                 # JSON output for scripting
```

### TUI

The TUI is launched via the CLI:

```bash
cargo run --bin nous -- terminal
```

### Web App

```bash
cd apps/web
npm install
npm run dev    # starts on http://localhost:3000
```

The web app expects the API server running on `http://localhost:8080`. Set `NEXT_PUBLIC_API_URL` to override.

### Desktop App (Tauri)

```bash
cd apps/desktop
nix-shell    # or install Tauri prerequisites manually
cargo tauri dev
```

## Project Structure

```
crates/
  nous-core         Core types, error handling, configuration
  nous-crypto       ed25519 signing, x25519 key exchange, AES-256-GCM, HKDF, Schnorr ZK proofs
  nous-identity     DID:key identifiers, verifiable credentials, reputation
  nous-net          libp2p networking — gossipsub, Kademlia DHT, relay, NAT traversal, mDNS
  nous-messaging    E2E encrypted messaging — channels, X3DH, Double Ratchet
  nous-social       Decentralized social — events, feeds, follow graph, reactions
  nous-governance   DAOs, proposals, quadratic voting, delegation, tallies
  nous-payments     Wallets, transfers, escrow, invoices
  nous-storage      SQLite-backed persistence with CRDT support
  nous-ai           Local AI inference, agent framework, semantic search
  nous-marketplace  P2P listings, purchases, reviews, ratings
  nous-browser      IPFS gateway, ENS resolution, dApp browser
  nous-files        Versioned file storage with content-addressed dedup, encrypted vaults
  nous-nostr        NIP-01/02/04/09 relay and client
  nous-api          REST + GraphQL + gRPC server (Axum, async-graphql, tonic)
  nous-tui          Terminal interface (ratatui)
  nous-cli          Command-line interface (clap) — binary name: nous
  nous-terminal     Embedded terminal emulator — PTY management, VT parsing
  nous-wasm         WebAssembly bindings for browser-side crypto
  nous-integration  Cross-crate integration tests
  nous-bench        Criterion benchmarks

apps/
  web/              Next.js PWA (offline-capable)
  desktop/          Tauri desktop app (Linux/macOS)
  android/          Kotlin + Jetpack Compose (UI shell)
  ios/              Swift + SwiftUI (UI shell)
```

## Configuration

Nous stores data in `~/.nous/` by default. There is no config file yet — configuration is done through struct defaults in code. Key defaults:

| Setting | Default | Notes |
|---|---|---|
| Data directory | `~/.nous/` | SQLite DB, keys, files |
| API host | `0.0.0.0` | API server bind address |
| API port | `8080` | REST + GraphQL |
| gRPC port | `8081` | API port + 1 |
| P2P listen | `0.0.0.0:9000` | libp2p TCP |
| Nostr relay | `:9735` | NIP-compliant WebSocket |
| Web app | `localhost:3000` | Next.js dev server |
| Log level | `info` | Override with `RUST_LOG` |
| Max peers | `50` | libp2p connection limit |
| CORS origins | `http://localhost:3001` | API server allowed origins |

### Environment Variables

| Variable | Description |
|---|---|
| `RUST_LOG` | Logging filter (e.g. `debug`, `nous_api=trace`) |
| `NEXT_PUBLIC_API_URL` | API base URL for the web app (default `http://localhost:8080/api/v1`) |
| `HOME` | Used to resolve `~/.nous/` data directory |

## Development

### Running Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p nous-crypto

# With output
cargo test --workspace -- --nocapture

# Benchmarks
cargo bench --workspace
```

### Code Quality

```bash
# Lint
cargo clippy --workspace -- -D warnings

# Format check
cargo fmt --check

# Format
cargo fmt
```

### Adding a New Crate

1. Create the crate under `crates/`
2. Add it to the `[workspace.members]` list in the root `Cargo.toml`
3. Use `version.workspace = true` and `edition.workspace = true` in its `Cargo.toml`
4. Add workspace dependencies rather than pinning versions locally

## License

MIT
