<div align="center">
  <img width="400" height="auto" src="docs/v2/public/images/SPACERCORD.png" alt="SpacerCord Logo">

  <h1>SpacerCord</h1>

  <p>High-performance Minecraft reverse proxy with integrated <b>SpacetimeDB</b> persistence.</p>

  <img alt="License" src="https://img.shields.io/badge/license-AGPL--3.0-blue?style=flat-square" />
  <a href="https://discord.gg/sqbJhZVSgG">
    <img alt="Discord" src="https://img.shields.io/discord/1330603066478825502?style=flat-square&label=discord" />
  </a>
</div>

<br />

## Infrarust Fork

SpacerCord is a specialized fork of [Infrarust](https://github.com/Shadowner/Infrarust). It inherits all the high-performance routing, zero-copy networking, and plugin architecture of the original project.

For standard proxy setup, routing configuration, Docker labels, and general usage, please refer to the **[Infrarust Documentation](https://infrarust.dev/v2/)**.

---

## SpacerCord Features

While maintaining 100% compatibility with Infrarust, SpacerCord introduces deep integration with **SpacetimeDB** to enable advanced persistence and cross-server logic directly within the proxy layer.

### 🚀 SpacetimeDB Persistence
- **Built-in Database SDK:** Persist player data, session history, and game state without managing external database instances.
- **Relational Logic:** Use SpacetimeDB's relational model to query proxy state, manage bans, or track player movement across your network.
- **Async Driver:** A dedicated background driver manages all database interactions, ensuring SpacetimeDB operations never block the networking hot path.

### 🛠️ Enhanced API
- **SpacetimeDB Event Hooks:** New event hooks allow plugins to automatically sync data to the database on specific proxy events (e.g., `PostLoginEvent`, `ServerSwitchEvent`).
- **Internal State Mirroring:** Proxy-wide state is mirrored into SpacetimeDB tables for easy external querying via CLI or Web API.

## SpacetimeDB Integration

SpacerCord bundles a customized SpacetimeDB SDK designed for the high-throughput requirements of a Minecraft proxy.

- **Persistent State:** Store player logins, play time, and cross-server data with transactional integrity.
- **Fire-and-Forget Reducers:** Call database logic via an MPSC bridge; the proxy continues processing packets while the database task completes.
- **Unified Schema:** Manage your proxy configuration and persistent game state in a single, cohesive environment.

### 📚 Documentation & Resources
- **[Official SpacetimeDB Docs](https://docs.spacetimedb.com)**
- **[Rust SDK Guide](https://docs.spacetimedb.com/getting-started/rust)**
- **[SpacerCord Discord](https://discord.gg/sqbJhZVSgG)**

### ⚙️ Setup Guide

#### Managed Setup (Recommended)
SpacerCord can automatically manage the SpacetimeDB process for you. Add this to your `infrarust.toml`:

```toml
[spacetimedb]
enabled = true
uri = "http://127.0.0.1:3000"
listen = "127.0.0.1:3000"
db_name = "spacer-cord"
auto_install = true
module_path = "./stdb-module"
```

#### Manual Setup
If you prefer to run SpacetimeDB independently (e.g., in a separate Docker container):
1. Install the CLI: `curl -L https://install.spacetimedb.com | sh`
2. Start the server: `spacetime start`
3. Update `infrarust.toml` to point to your instance (`enabled = true`, `auto_install = false`).

### 📦 Module Development

The proxy logic resides in the `stdb-module/` directory. This is a standard SpacetimeDB module written in Rust.

1. **Define Schema:** Add `#[table]` structs in `stdb-module/src/lib.rs`.
2. **Define Logic:** Create `#[reducer]` functions to handle data.
3. **Publish:** If using `auto_install`, SpacerCord publishes the module on startup. Manually: `spacetime publish -s http://localhost:3000 <module_name>`.

### 🔌 Plugin Development

Plugins interact with SpacetimeDB through the `SpacetimeService` provided in the plugin context.

#### Calling a Reducer
```rust
fn on_login(&self, ctx: &PluginContext, event: &PostLoginEvent) {
    let stdb = ctx.spacetimedb();
    // Fire-and-forget: returns immediately, executed on driver thread
    stdb.ensure_player_profile(event.player.uuid().to_string(), event.player.username().to_string());
}
```

#### Subscribing to Changes
```rust
fn on_enable(&self, ctx: &PluginContext) {
    // Subscribe to all changes in the "player_profile" table
    ctx.spacetimedb().subscribe("SELECT * FROM player_profile");
}

// React to the event
fn on_stdb_row(&self, ctx: &PluginContext, event: &SpacetimeRowEvent) {
    if event.table_name == "player_profile" {
        info!("Player profile updated: {:?}", event.operation);
    }
}
```

## Quick Start

### Build from Source
Ensure you have Rust 1.94+ installed.

```bash
git clone https://github.com/Shadowner/SpacerCord.git
cd SpacerCord
cargo build --release -p infrarust
```

### Run

```bash
./target/release/infrarust --config infrarust.toml
```

## Contributing & License

Contributions to the SpacerCord integration are welcome. For core proxy changes, we recommend contributing directly to [Infrarust](https://github.com/Shadowner/Infrarust).

Licensed under **AGPL-3.0** with plugin exceptions. See [LICENSE](LICENSE) for details.

<p align="center">
  <img height="60" src="docs/v1/public/img/agplv3_logo.svg" alt="AGPL v3" />
</p>
