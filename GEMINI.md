# GEMINI.md - SpacerCord Project Context

This file provides architectural context, development guidelines, and operational procedures for **SpacerCord**, a high-performance Minecraft reverse proxy.

## Project Overview

SpacerCord is a specialized fork of [Infrarust](https://github.com/Shadowner/Infrarust). It is a Minecraft reverse proxy written in Rust that routes players to backend servers based on domain names. 

### Key Features
- **Domain Routing:** Map subdomains (e.g., `survival.example.com`) to specific backend IPs/ports.
- **Proxy Modes:** Supports `passthrough`, `zerocopy`, `client_only`, `offline`, and `server_only`.
- **SpacetimeDB Integration:** The primary goal of this fork is to bundle the SpacetimeDB SDK, allowing network operators to persist game state and player data directly within the proxy.
- **Plugin System:** A 3-tier event-driven architecture for extending proxy behavior.
- **Observability:** Built-in OpenTelemetry support and a web dashboard/API.

## Technical Stack
- **Language:** Rust (Edition 2024, MSRV 1.94)
- **Runtime:** Tokio (Multi-threaded async)
- **Networking:** Custom protocol implementation (`infrarust_protocol`), `tokio-util` codecs.
- **Serialization:** `serde`, `toml`, `fastnbt`.
- **Web UI:** Axum (Backend API) + VitePress (Documentation).

## Workspace Architecture

The project is organized as a Cargo workspace:

### Core Crates (`crates/`)
- `infrarust`: The CLI entry point and orchestration layer.
- `infrarust-core`: Main proxy logic, session management, routing, and event bus.
- `infrarust-api`: The stable trait/event surface used by plugins. **Logic-free definitions.**
- `infrarust_protocol`: Minecraft packet codec, encryption (AES-CFB8), and NBT handling.
- `infrarust_config`: TOML configuration parsing and hot-reloading logic.
- `infrarust-transport`: Low-level TCP handling and PROXY protocol support.
- `infrarust_server_manager`: Backend server lifecycle and health monitoring.

### Plugins (`plugins/`)
- `infrarust-plugin-admin-api`: REST API and embedded Web Dashboard.
- `infrarust-plugin-auth`: Handles `/login` and `/register` in a "Limbo" state.
- `infrarust-plugin-hello`: A minimal template for new plugin development.

### Tools (`tools/`)
- `stress-test`: Utility for benchmarking proxy performance.
- `registry-extractor`: Tool for extracting Minecraft protocol data.

## Development Workflow

### Key Commands
```bash
# Build the project
cargo build

# Run in development with a specific config
cargo run --bin infrarust -- --config infrarust.toml

# Run all tests
cargo test

# Check for linting issues
cargo clippy

# Format code
cargo fmt
```

### Feature Flags
- `default-plugins`: Includes auth and server-wake.
- `telemetry`: Enables OTLP export.
- `docker`: Enables Docker auto-discovery provider.

## Plugin Architecture

Plugins interact with the proxy through three capability tiers defined in `infrarust-api`:

1.  **Tier 1 (Event Listeners):** Subscribe to `PostLoginEvent`, `ChatEvent`, etc., via the `EventBus`.
2.  **Tier 2 (Limbo Handlers):** Intercept players in a virtual world (e.g., for authentication or queueing).
3.  **Tier 3 (Virtual Backends):** Speak raw Minecraft protocol to host mini-games entirely inside the proxy.

## SpacetimeDB Integration Strategy

To maintain high performance, SpacetimeDB interactions follow an async-safe pattern:
- **Dedicated Thread:** A `stdb-driver` OS thread manages the synchronous SpacetimeDB connection.
- **MPSC Bridge:** Tokio tasks send messages to the driver thread via a bounded `mpsc::sync_channel`.
- **Fire-and-Forget:** Reducers are called by sending a message to the channel; results are typically handled via callbacks or state updates.

## Coding Standards

- **Unsafe Code:** Discouraged; `unsafe_code = warn` is enforced at the workspace level.
- **Error Handling:** Use `thiserror` for library crates and `anyhow` for the CLI.
- **Commits:** Follow **Conventional Commits** (`feat:`, `fix:`, `docs:`, `refactor:`).
- **Performance:** Focus on zero-copy or minimal-allocation packet processing in the hot path.
