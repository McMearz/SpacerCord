# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working in this repository.

## Project Intent

SpacerCord is a public fork of [Infrarust](https://github.com/Shadowner/Infrarust), a high-performance Minecraft reverse proxy written in Rust. The fork bundles the SpacetimeDB SDK directly into the proxy and exposes it to the plugin API, so Minecraft network operators can persist player data, balances, and game state in SpacetimeDB without writing the SDK integration themselves. Commercial plugins that depend on SpacerCord (and thus on SpacetimeDB) are sold separately for profit.

The SpacetimeDB integration work-in-progress lives at `D:\Minecraft\infrarust`. That directory has the `crates/infrarust-spacetimedb` crate that must be ported into this repo before anything else SpacetimeDB-related can be added here.

## Commands

```bash
# Build
cargo build

# Build release
cargo build --release

# Run (development)
cargo run --bin infrarust -- --config-path infrarust.toml

# Run tests (all workspace)
cargo test

# Run tests for a single crate
cargo test -p infrarust-core

# Lint
cargo clippy

# Format
cargo fmt
```

Feature flags used in builds:
- `--features default-plugins` — includes auth + server-wake plugins
- `--features telemetry` — enables OpenTelemetry/OTLP export
- `--features docker` — enables Docker provider

## Workspace Layout

```
crates/
  infrarust_protocol      Minecraft protocol codec (fastnbt, AES-CFB8, RSA)
  infrarust-api           The plugin API surface: traits, events, services, filters
  infrarust_config        TOML/YAML config parsing and hot-reload
  infrarust-core          Proxy logic, event bus, handlers, session, routing
  infrarust_server_manager  Server lifecycle management
  infrarust-transport     Low-level TCP + PROXY protocol v2
  infrarust               CLI entry point

plugins/
  infrarust-plugin-auth       Password /login /register with limbo handler
  infrarust-plugin-hello      Minimal plugin example (start here for new plugins)
  infrarust-plugin-admin-api  Axum HTTP server with REST API + embedded Web UI

tools/
  infrarust-motd, registry-extractor, stress-test
```

The **SpacetimeDB crate to add**: `crates/infrarust-spacetimedb` (port from `D:\Minecraft\infrarust\crates\infrarust-spacetimedb`).

## Plugin Architecture (Three Tiers)

All plugin entry points implement the `Plugin` trait in `crates/infrarust-api/src/plugin.rs`. A `PluginContext` (sealed, proxy-only) provides access to all services. Call `ctx.event_bus()` to subscribe, `ctx.player_registry()` to look up players, etc.

**Tier 1 — Event listener.** Subscribe to typed events on the `EventBus` with priority `FIRST/EARLY/NORMAL/LATE/LAST`. Use for cross-cutting concerns (e.g. fire a SpacetimeDB reducer on `PostLoginEvent`).

**Tier 2 — Limbo handler.** Implement `LimboHandler` (`crates/infrarust-api/src/limbo/`). The proxy keeps the client connected in a fake world while your handler runs game logic. `on_player_enter()` returns `HandlerResult::{Accept, Deny, Hold, Redirect}`. `on_command()` and `on_chat()` let you drive a text UI. The auth plugin is the canonical example.

**Tier 3 — Virtual backend.** Implement `VirtualBackendHandler` (`crates/infrarust-api/src/virtual_backend/`). Your plugin speaks raw Minecraft protocol packets directly to the client — you must send `JoinGame` + chunk data in `on_session_start()`. Use for mini-games or lobbies hosted entirely inside the proxy process.

## SpacetimeDB Integration Pattern

The integration in `D:\Minecraft\infrarust\crates\infrarust-spacetimedb\src\lib.rs` solves the async/threading mismatch:

- A **dedicated OS thread** (`stdb-driver`) owns the `DbConnection` and runs `conn.run_threaded()` (the SDK's internal WebSocket loop).
- A bounded `mpsc::sync_channel(1024)` bridges the Tokio world to that OS thread, providing back-pressure when SpacetimeDB is slow.
- `SpacetimeHandle` is `Clone` (cheap — just clones the `SyncSender`) so it can be shared across event handlers.
- Reducer calls are **fire-and-forget**: `try_send` queues the message and returns immediately; the driver thread serialises all outgoing WS writes.

To add a new reducer call:
1. Add a variant to `DriverMsg`.
2. Add a method on `SpacetimeHandle` that does `self.tx.try_send(...)`.
3. Handle the variant in the `while let Ok(msg) = rx.recv()` loop inside `driver_main`.
4. Expose the handle via a SpacerCord service so plugins can call it from `PluginContext`.

Module bindings (`src/module_bindings/`) are generated from the SpacetimeDB schema and should not be edited by hand.

## Services Available to Plugins

All accessed via `PluginContext` (sealed — only usable inside the proxy process):

| Service | Purpose |
|---|---|
| `PlayerRegistry` | Look up online players by username / UUID / internal ID |
| `ServerManager` | Get/start/stop backend servers, register state-change callbacks |
| `BanService` | Ban/unban/check by UUID, IP, or username |
| `ConfigService` | Query server configs and proxy settings |
| `Scheduler` | Schedule and cancel async tasks |
| `PluginRegistry` | Query metadata and state of other plugins |

The SpacetimeDB handle will be exposed as an additional service once the crate is ported.

## Filter System

**Codec filters** (`crates/infrarust-api/src/filter/codec.rs`): run inline on every packet, synchronously. Must complete in under ~1 µs. Can modify, inject, or drop packets and react to state transitions (handshake → login → play). Implement `CodecFilterFactory` + `CodecFilterInstance`.

**Transport filters** (`crates/infrarust-api/src/filter/transport.rs`): operate at the TCP layer, before packet framing.

## Coding Standards

- Rust edition 2024, MSRV 1.94.
- `unsafe_code = warn` workspace-wide — keep it that way.
- Commit style: Conventional Commits (`feat:`, `fix:`, `patch:`, `docs:`).
- `cargo clippy` must pass before committing.
- Release profile uses `lto = "fat"` + `codegen-units = 1`; expect slow release builds.
